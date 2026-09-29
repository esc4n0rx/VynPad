//! A linha de comando do `padgaming-sim`.

use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use clap::{Args, Parser, Subcommand};

use crate::client::{self, ClientConfig, ClientReport, Event};
use crate::link::FaultConfig;
use crate::report::{BenchParams, BenchReport};
use crate::source::{InputSource, Pattern, PatternKind, Script};
use crate::units::{parse_duration, parse_percent};
use crate::{DEV_KEY, DEV_SESSION_ID};

/// Porta UDP de input do server.
const DEFAULT_PORT: u16 = 47801;

/// Simulador de client do `PadGaming`.
#[derive(Debug, Parser)]
#[command(name = "padgaming-sim", version, about)]
pub struct Cli {
    /// O que fazer.
    #[command(subcommand)]
    pub command: Command,
}

/// Subcomandos.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Lista servers na rede (mDNS/broadcast). Requer a descoberta do server (T-303).
    Discover,
    /// Pareia com um server e guarda o `device_token`. Requer o canal TLS (T-302).
    Pair {
        /// URI do QR code (`padgaming://pair?...`).
        #[arg(long, conflicts_with = "pin")]
        uri: Option<String>,
        /// PIN de 6 dígitos (use com `--host`).
        #[arg(long)]
        pin: Option<String>,
        /// Endereço do server (com `--pin`).
        #[arg(long)]
        host: Option<String>,
    },
    /// Envia input simulado a um server.
    Connect(ConnectArgs),
    /// Gera carga com vários clients e mede o RTT.
    Bench(BenchArgs),
}

/// Onde e como conectar.
#[derive(Debug, Args)]
pub struct Target {
    /// Endereço do server: `IP` ou `IP:porta` (porta padrão 47801).
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// Usa a sessão de desenvolvimento (sessão e chave fixas e públicas). Só funciona contra um
    /// server de debug com o modo dev ligado (`cargo run -p padgaming-net --example dev_server`).
    #[arg(long)]
    pub insecure_dev: bool,
}

/// Falhas de rede a simular.
#[derive(Debug, Args, Clone, Copy)]
pub struct FaultArgs {
    /// Perda de pacotes, ex.: `10%`.
    #[arg(long, value_parser = parse_percent, default_value = "0%")]
    pub loss: f64,
    /// Atraso fixo, ex.: `20ms`.
    #[arg(long, value_parser = parse_duration, default_value = "0ms")]
    pub delay: Duration,
    /// Jitter (atraso aleatório de 0 até o valor), ex.: `5ms`.
    #[arg(long, value_parser = parse_duration, default_value = "0ms")]
    pub jitter: Duration,
    /// Atrasa parte dos pacotes para chegarem fora de ordem.
    #[arg(long)]
    pub reorder: bool,
}

impl From<FaultArgs> for FaultConfig {
    fn from(a: FaultArgs) -> Self {
        Self {
            loss: a.loss,
            delay: a.delay,
            jitter: a.jitter,
            reorder: a.reorder,
        }
    }
}

/// Argumentos de `connect`.
#[derive(Debug, Args)]
pub struct ConnectArgs {
    /// Server.
    #[command(flatten)]
    pub target: Target,
    /// Padrão pronto: `circle`, `buttons` ou `stress`.
    #[arg(long, conflicts_with_all = ["script", "gamepad", "interactive"])]
    pub pattern: Option<PatternKind>,
    /// Sequência de estados com tempos (arquivo TOML).
    #[arg(long, conflicts_with_all = ["gamepad", "interactive"])]
    pub script: Option<PathBuf>,
    /// Repassa um controle físico local (requer o build com `--features gamepad`).
    #[arg(long, conflicts_with = "interactive")]
    pub gamepad: bool,
    /// Teclado → botões (ainda não disponível).
    #[arg(long)]
    pub interactive: bool,
    /// Pacotes de input por segundo.
    #[arg(long, default_value_t = 250)]
    pub rate: u32,
    /// Para depois deste tempo (padrão: até Ctrl+C).
    #[arg(long, value_parser = parse_duration)]
    pub duration: Option<Duration>,
    /// Falhas de rede simuladas.
    #[command(flatten)]
    pub fault: FaultArgs,
    /// Semente das falhas e do padrão `stress` (padrão: aleatória).
    #[arg(long)]
    pub seed: Option<u64>,
}

/// Argumentos de `bench`.
#[derive(Debug, Args)]
pub struct BenchArgs {
    /// Server.
    #[command(flatten)]
    pub target: Target,
    /// Clients simultâneos (cada um com sua sessão, a partir de 1).
    #[arg(long, default_value_t = 4)]
    pub clients: usize,
    /// Pacotes de input por segundo, por client.
    #[arg(long, default_value_t = 250)]
    pub rate: u32,
    /// Duração da carga.
    #[arg(long, value_parser = parse_duration, default_value = "10s")]
    pub duration: Duration,
    /// Grava o relatório em JSON neste arquivo.
    #[arg(long)]
    pub json: Option<PathBuf>,
    /// Falhas de rede simuladas.
    #[command(flatten)]
    pub fault: FaultArgs,
    /// Semente das falhas.
    #[arg(long)]
    pub seed: Option<u64>,
}

/// Resolve `IP` ou `IP:porta` (porta padrão 47801), preferindo IPv4.
pub fn resolve_host(host: &str) -> anyhow::Result<SocketAddr> {
    let candidates: Vec<SocketAddr> = match host.parse::<SocketAddr>() {
        Ok(addr) => vec![addr],
        Err(_) => (host, DEFAULT_PORT)
            .to_socket_addrs()
            .with_context(|| format!("não consegui resolver `{host}`"))?
            .collect(),
    };
    candidates
        .iter()
        .copied()
        .find(SocketAddr::is_ipv4)
        .or_else(|| candidates.first().copied())
        .with_context(|| format!("`{host}` não resolveu para nenhum endereço"))
}

fn require_dev(target: &Target) -> anyhow::Result<()> {
    if !target.insecure_dev {
        bail!(
            "sem o pareamento (T-302) a única forma de autenticar é `--insecure-dev`, contra um \
             server de debug com o modo dev ligado"
        );
    }
    Ok(())
}

fn random_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64)
}

fn install_ctrlc() -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    // Se já houver um handler (testes), seguimos sem Ctrl+C.
    let _ = ctrlc::set_handler(move || flag.store(true, Ordering::Relaxed));
    stop
}

fn make_source(args: &ConnectArgs, seed: u64) -> anyhow::Result<Box<dyn InputSource>> {
    if args.interactive {
        bail!(
            "`--interactive` (TUI) ainda não está disponível; use `--pattern`, `--script` ou `--gamepad`"
        );
    }
    if args.gamepad {
        #[cfg(feature = "gamepad")]
        {
            let pad = crate::source::Gamepad::new()?;
            let names = pad.connected();
            if names.is_empty() {
                eprintln!("aviso: nenhum controle conectado; enviando estado neutro até ligar um");
            } else {
                eprintln!("controle: {}", names.join(", "));
            }
            return Ok(Box::new(pad));
        }
        #[cfg(not(feature = "gamepad"))]
        bail!(
            "este build não tem `--gamepad`; compile com `cargo run -p padgaming-sim --features gamepad`"
        );
    }
    if let Some(path) = &args.script {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("não consegui ler {}", path.display()))?;
        return Ok(Box::new(Script::parse(&text)?));
    }
    Ok(Box::new(Pattern::new(
        args.pattern.unwrap_or(PatternKind::Circle),
        seed,
    )))
}

fn connect(args: &ConnectArgs) -> anyhow::Result<()> {
    require_dev(&args.target)?;
    let seed = args.seed.unwrap_or_else(random_seed);
    let server = resolve_host(&args.target.host)?;
    let mut source = make_source(args, seed)?;
    let config = ClientConfig {
        server,
        session_id: DEV_SESSION_ID,
        key: DEV_KEY,
        rate_hz: args.rate,
        fault: args.fault.into(),
        seed,
    };
    println!(
        "conectando a {server} a {} Hz (Ctrl+C para parar)",
        args.rate
    );
    let stop = install_ctrlc();
    let report = client::run(
        &config,
        source.as_mut(),
        &stop,
        args.duration,
        &mut |event| match event {
            Event::Rumble { large, small } => {
                println!("rumble: grande {large:3}  pequeno {small:3}");
            }
            Event::Status {
                elapsed,
                inputs_sent,
                pongs,
                last_rtt_us,
            } => println!(
                "{:>5.0}s  inputs {inputs_sent}  pongs {pongs}  rtt {}",
                elapsed.as_secs_f64(),
                last_rtt_us.map_or("-".to_owned(), |us| format!(
                    "{:.2} ms",
                    f64::from(us) / 1000.0
                ))
            ),
        },
    )?;
    let summary = crate::report::Summary::of(&report);
    println!(
        "\nfim: {} inputs, {} pings ({} pongs, {:.1}% sem resposta), rtt p50 {:.2} ms / p99 {:.2} ms, {} rumbles",
        summary.inputs_sent,
        summary.pings_sent,
        summary.pongs,
        summary.ping_loss_percent,
        f64::from(summary.rtt_us.p50) / 1000.0,
        f64::from(summary.rtt_us.p99) / 1000.0,
        summary.rumbles
    );
    Ok(())
}

fn bench(args: &BenchArgs) -> anyhow::Result<()> {
    require_dev(&args.target)?;
    if args.clients == 0 || args.clients > 4 {
        bail!("`--clients` deve estar entre 1 e 4 (são 4 slots)");
    }
    let seed = args.seed.unwrap_or_else(random_seed);
    let server = resolve_host(&args.target.host)?;
    println!(
        "bench: {} clients x {} Hz por {:?} contra {server}",
        args.clients, args.rate, args.duration
    );
    let stop = install_ctrlc();
    let started = Instant::now();
    let threads: Vec<_> = (0..args.clients)
        .map(|i| {
            let config = ClientConfig {
                server,
                session_id: DEV_SESSION_ID + u32::try_from(i).unwrap_or(0),
                key: DEV_KEY,
                rate_hz: args.rate,
                fault: args.fault.into(),
                seed: seed.wrapping_add(i as u64),
            };
            let (stop, duration) = (Arc::clone(&stop), args.duration);
            std::thread::Builder::new()
                .name(format!("sim-{}", i + 1))
                .spawn(move || -> anyhow::Result<ClientReport> {
                    let mut source = Pattern::new(PatternKind::Stress, config.seed);
                    client::run(&config, &mut source, &stop, Some(duration), &mut |_| {})
                })
                .expect("thread do client")
        })
        .collect();
    let mut reports = Vec::new();
    for thread in threads {
        reports.push(
            thread
                .join()
                .map_err(|_| anyhow::anyhow!("um client deu pânico"))??,
        );
    }

    let params = BenchParams {
        server: server.to_string(),
        clients: args.clients,
        rate_hz: args.rate,
        duration_s: args.duration.as_secs_f64(),
        loss_percent: args.fault.loss * 100.0,
        delay_ms: args.fault.delay.as_secs_f64() * 1000.0,
        jitter_ms: args.fault.jitter.as_secs_f64() * 1000.0,
        reorder: args.fault.reorder,
    };
    let report = BenchReport::new(params, &reports, started.elapsed());
    println!("\n{}", report.to_text());
    if let Some(path) = &args.json {
        std::fs::write(path, report.to_json())
            .with_context(|| format!("não consegui gravar {}", path.display()))?;
        println!("relatório JSON: {}", path.display());
    }
    if report.total.pongs == 0 {
        bail!("nenhum Pong chegou: o server está de pé e com o modo dev ligado?");
    }
    Ok(())
}

/// Executa o comando.
pub fn run(cli: &Cli) -> anyhow::Result<()> {
    match &cli.command {
        Command::Discover => bail!("`discover` depende da descoberta mDNS do server (T-303)"),
        Command::Pair { .. } => bail!("`pair` depende do canal de controle TLS do server (T-302)"),
        Command::Connect(args) => connect(args),
        Command::Bench(args) => bench(args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("padgaming-sim").chain(args.iter().copied()))
    }

    #[test]
    fn connect_parses_all_options() {
        let cli = parse(&[
            "connect",
            "--host",
            "10.0.0.2:5000",
            "--insecure-dev",
            "--pattern",
            "stress",
            "--rate",
            "125",
            "--duration",
            "5s",
            "--loss",
            "10%",
            "--delay",
            "20ms",
            "--jitter",
            "5ms",
            "--reorder",
            "--seed",
            "7",
        ])
        .unwrap();
        let Command::Connect(a) = cli.command else {
            panic!("esperava connect")
        };
        assert_eq!(a.target.host, "10.0.0.2:5000");
        assert!(a.target.insecure_dev);
        assert_eq!(a.pattern, Some(PatternKind::Stress));
        assert_eq!(
            (a.rate, a.duration, a.seed),
            (125, Some(Duration::from_secs(5)), Some(7))
        );
        assert!((a.fault.loss - 0.1).abs() < 1e-12);
        assert_eq!(
            (a.fault.delay, a.fault.jitter),
            (Duration::from_millis(20), Duration::from_millis(5))
        );
        assert!(a.fault.reorder);
    }

    #[test]
    fn defaults_are_sensible() {
        let Command::Connect(a) = parse(&["connect"]).unwrap().command else {
            panic!()
        };
        assert_eq!(
            (a.target.host.as_str(), a.rate, a.duration),
            ("127.0.0.1", 250, None)
        );
        assert!(!a.target.insecure_dev);
        let Command::Bench(b) = parse(&["bench"]).unwrap().command else {
            panic!()
        };
        assert_eq!(
            (b.clients, b.rate, b.duration),
            (4, 250, Duration::from_secs(10))
        );
        assert!(FaultConfig::from(b.fault).is_clean());
    }

    #[test]
    fn conflicting_or_invalid_options_are_refused() {
        assert!(parse(&["connect", "--pattern", "circle", "--script", "x.toml"]).is_err());
        assert!(parse(&["connect", "--gamepad", "--interactive"]).is_err());
        assert!(parse(&["connect", "--pattern", "zigzag"]).is_err());
        assert!(parse(&["connect", "--loss", "10"]).is_err());
        assert!(parse(&["connect", "--delay", "abc"]).is_err());
        assert!(parse(&["bench", "--clients", "x"]).is_err());
        assert!(parse(&["pair", "--uri", "u", "--pin", "1"]).is_err());
    }

    #[test]
    fn hosts_resolve_with_and_without_a_port() {
        assert_eq!(
            resolve_host("127.0.0.1").unwrap(),
            "127.0.0.1:47801".parse().unwrap()
        );
        assert_eq!(
            resolve_host("192.168.0.5:9000").unwrap(),
            "192.168.0.5:9000".parse().unwrap()
        );
        assert!(resolve_host("isto não é um host").is_err());
    }

    #[test]
    fn commands_that_need_later_tasks_say_so() {
        let err = |args: &[&str]| format!("{:#}", run(&parse(args).unwrap()).unwrap_err());
        assert!(err(&["discover"]).contains("T-303"));
        assert!(err(&["pair", "--pin", "123456"]).contains("T-302"));
        assert!(err(&["connect"]).contains("--insecure-dev"));
        assert!(err(&["bench", "--insecure-dev", "--clients", "9"]).contains("entre 1 e 4"));
        assert!(err(&["connect", "--insecure-dev", "--interactive"]).contains("interactive"));
        #[cfg(not(feature = "gamepad"))]
        assert!(err(&["connect", "--insecure-dev", "--gamepad"]).contains("features gamepad"));
        assert!(
            err(&["connect", "--insecure-dev", "--script", "nao-existe.toml"])
                .contains("nao-existe.toml")
        );
    }
}
