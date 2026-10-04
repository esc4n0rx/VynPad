//! `pair`: o simulador faz o pareamento de verdade, pelo canal de controle TLS (T-302), como o
//! app do celular fará. Serve para testar o server de ponta a ponta sem o celular, inclusive nos
//! testes E2E da interface (T-701), que leem o PIN na tela e o digitam aqui.

use std::fmt::Write as _;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use clap::Args;
use serde_json::json;
use vynpad_protocol::Secret;
use vynpad_protocol::control::{Auth, Bye, ByeReason, Bytes, ControlMessage, Uuid};
use vynpad_protocol::pairing_uri::PairingUri;

use crate::client::{self, ClientConfig};
use crate::control::{self, ControlSession, Identity, Outcome, Trust};
use crate::link::FaultConfig;
use crate::source::{Pattern, PatternKind};
use crate::units::parse_duration;

/// Porta TCP de controle padrão do server.
const CONTROL_PORT: u16 = 47800;

/// Argumentos de `pair`.
#[derive(Debug, Args)]
pub struct PairArgs {
    /// URI do QR code (`vynpad://pair?...`); traz endereço, porta, fingerprint e token.
    #[arg(long, conflicts_with_all = ["pin", "host"])]
    pub uri: Option<String>,
    /// PIN de 6 dígitos mostrado no server (use com `--host`).
    #[arg(long, requires = "host")]
    pub pin: Option<String>,
    /// Server: `IP` ou `IP:porta` (porta de controle padrão 47800).
    #[arg(long)]
    pub host: Option<String>,
    /// Nome exibido no server.
    #[arg(long, default_value = "VynPad Sim")]
    pub name: String,
    /// `device_id` (UUID minúsculo). Padrão: um novo.
    #[arg(long)]
    pub id: Option<String>,
    /// Depois de parear, mantém a sessão (controle e input) por este tempo, com o controle
    /// ocupando um slot. Sem isso, a sessão fecha logo após o pareamento.
    #[arg(long, value_parser = parse_duration)]
    pub hold: Option<Duration>,
    /// Grava o resultado do pareamento (JSON, com o token do dispositivo) neste arquivo.
    #[arg(long)]
    pub save: Option<PathBuf>,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// Onde conectar, em quem confiar e como se autenticar.
fn plan(args: &PairArgs) -> anyhow::Result<(SocketAddr, Trust, Auth)> {
    if let Some(uri) = &args.uri {
        let uri = PairingUri::parse(uri).map_err(|e| anyhow::anyhow!("URI inválida: {e}"))?;
        let host = *uri.hosts.first().context("a URI não traz endereço")?;
        return Ok((
            SocketAddr::new(IpAddr::V4(host), uri.port),
            Trust::Pinned(uri.fingerprint),
            Auth::PairingToken {
                token: Secret::new(Bytes(*uri.pairing_token.expose())),
            },
        ));
    }
    let (Some(pin), Some(host)) = (&args.pin, &args.host) else {
        bail!("informe `--uri`, ou `--pin` junto com `--host`");
    };
    let addr = match host.parse::<SocketAddr>() {
        Ok(addr) => addr,
        Err(_) => (host.as_str(), CONTROL_PORT)
            .to_socket_addrs()
            .with_context(|| format!("não consegui resolver `{host}`"))?
            .find(SocketAddr::is_ipv4)
            .with_context(|| format!("`{host}` não resolveu para um IPv4"))?,
    };
    // O PIN vale dentro do TLS, mas o certificado só é conhecido depois: confia no primeiro e
    // mostra o fingerprint para conferência, como o app faz no pareamento por PIN.
    Ok((
        addr,
        Trust::FirstUse,
        Auth::Pin {
            pin: Secret::new(pin.clone()),
        },
    ))
}

/// Executa `pair`.
pub fn run(args: &PairArgs) -> anyhow::Result<()> {
    let (addr, trust, auth) = plan(args)?;
    let device_id = match &args.id {
        Some(id) => Uuid::parse(id).map_err(|e| anyhow::anyhow!("--id: {e}"))?,
        None => Uuid::parse(&crate::cli::random_uuid()).map_err(|e| anyhow::anyhow!("{e}"))?,
    };
    let mut identity = Identity::simulated(device_id.clone());
    identity.name.clone_from(&args.name);

    let mut session = match control::connect(addr, trust, &identity, auth)? {
        Outcome::Welcome(session) => session,
        Outcome::Rejected(reject) => bail!("o server recusou: {:?}", reject.reason),
    };
    let welcome = &session.welcome;
    let fingerprint = hex(&session.fingerprint);
    println!(
        "pareado com `{}` no slot P{} (fingerprint {fingerprint})",
        welcome.server_name,
        welcome.config.slot + 1
    );

    if let Some(path) = &args.save {
        let saved = json!({
            "device_id": device_id.as_str(),
            "server": addr.to_string(),
            "server_id": welcome.server_id.as_str(),
            "fingerprint": fingerprint,
            "device_token": welcome.device_token.as_ref().map(|t| hex(&t.expose().0)),
        });
        std::fs::write(path, serde_json::to_string_pretty(&saved)?)
            .with_context(|| format!("não consegui gravar {}", path.display()))?;
        println!("resultado gravado em {}", path.display());
    }

    if let Some(hold) = args.hold {
        hold_session(&mut session, addr, hold)?;
    }
    Ok(())
}

/// Mantém a sessão viva por `hold`: input pela UDP e keepalive no canal de controle.
fn hold_session(
    session: &mut ControlSession,
    addr: SocketAddr,
    hold: Duration,
) -> anyhow::Result<()> {
    let welcome = &session.welcome;
    let config = ClientConfig {
        server: SocketAddr::new(addr.ip(), welcome.udp_port),
        session_id: welcome.session_id,
        key: welcome.udp_key.expose().0,
        rate_hz: 125,
        fault: FaultConfig::default(),
        seed: u64::from(welcome.session_id),
    };
    let stop = Arc::new(AtomicBool::new(false));
    let sender = {
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut source = Pattern::new(PatternKind::Buttons, config.seed);
            client::run(&config, &mut source, &stop, None, &mut |_| {})
        })
    };
    println!("segurando a sessão por {hold:?}");

    let start = Instant::now();
    while start.elapsed() < hold {
        match session.recv(Duration::from_millis(500)) {
            Ok(ControlMessage::Ping) => session.send(&ControlMessage::Pong)?,
            Ok(ControlMessage::Bye(_)) => break,
            Ok(_) => {}
            Err(error) => {
                // Sem mensagem no intervalo é normal; qualquer outra falha encerra.
                let timed_out = error.downcast_ref::<std::io::Error>().is_some_and(|e| {
                    matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    )
                });
                if !timed_out {
                    break;
                }
            }
        }
    }
    stop.store(true, Ordering::Relaxed);
    let _ = session.send(&ControlMessage::Bye(Bye {
        reason: ByeReason::User,
        message: None,
    }));
    sender
        .join()
        .map_err(|_| anyhow::anyhow!("o envio de input deu pânico"))??;
    Ok(())
}
