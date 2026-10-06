//! Canal de controle do client (PROTOCOLO §5): TLS 1.3 com o certificado do server confiado pelo
//! fingerprint, `hello` e `welcome`. Bloqueante: o app chama fora da thread principal.

use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{WebPkiSupportedAlgorithms, ring};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, ClientConnection, DigitallySignedStruct, SignatureScheme, StreamOwned};
use sha2::{Digest, Sha256};
use vynpad_protocol::Secret;
use vynpad_protocol::control::{
    Auth, Bye, ByeReason, Bytes, Caps, ControlMessage, Hello, Incoming, RejectReason, Uuid,
    decode_body, encode_frame, frame_len,
};

use crate::CoreError;

/// Nome usado no SNI; o certificado do server é confiado pelo fingerprint, não pelo nome.
const SERVER_NAME: &str = "vynpad.local";
/// Prazo para abrir a conexão TCP (a UI tenta um endereço por vez).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(4);
/// Prazo de escrita e da leitura do `welcome`: o server pode esperar a aprovação do usuário no PC
/// (até 30 s) antes de responder.
const IO_TIMEOUT: Duration = Duration::from_secs(35);
/// Bytes do fingerprint que o mDNS e o broadcast anunciam (`fp`).
const SHORT_FP_LEN: usize = 8;

/// Em quem confiar no TLS.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum TrustPolicy {
    /// Só o certificado com este fingerprint inteiro (SHA-256 do DER, 32 bytes): o pinning de
    /// quem já pareou e o do QR code.
    Pinned {
        /// SHA-256 do certificado.
        fingerprint: Vec<u8>,
    },
    /// Só um certificado cujo fingerprint comece por estes 8 bytes (16 caracteres hex), os que a
    /// descoberta anuncia. Usado no PIN: pega um server diferente do que foi descoberto.
    ShortFingerprint {
        /// Hex minúsculo, 16 caracteres.
        hex: String,
    },
    /// Confia no primeiro certificado (PIN sem descoberta); o fingerprint visto fica no `welcome`.
    FirstUse,
}

/// Credencial do `hello`.
#[derive(Clone, PartialEq, Eq, uniffi::Enum)]
pub enum Credential {
    /// Token de uso único do QR code (16 bytes).
    PairingToken {
        /// O token.
        token: Vec<u8>,
    },
    /// PIN de 6 dígitos mostrado no server.
    Pin {
        /// O PIN em texto (preserva zeros à esquerda).
        pin: String,
    },
    /// Token de um dispositivo já pareado (32 bytes).
    DeviceToken {
        /// O token.
        token: Vec<u8>,
    },
}

// Os segredos nunca vão para o log por engano.
impl std::fmt::Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::PairingToken { .. } => "Credential::PairingToken(..)",
            Self::Pin { .. } => "Credential::Pin(..)",
            Self::DeviceToken { .. } => "Credential::DeviceToken(..)",
        })
    }
}

/// O que o aparelho anuncia no `hello`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DeviceInfo {
    /// UUID minúsculo estável do aparelho.
    pub device_id: String,
    /// Nome exibido no server (1–64).
    pub name: String,
    /// Modelo (≤ 64).
    pub model: String,
    /// Versão do sistema (≤ 32).
    pub os_ver: String,
    /// Versão do app (≤ 32).
    pub app_ver: String,
    /// Tem giroscópio/acelerômetro.
    pub gyro: bool,
    /// Consegue vibrar.
    pub rumble: bool,
}

/// O `welcome`, já em tipos simples.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct WelcomeInfo {
    /// Sessão UDP.
    pub session_id: u32,
    /// Porta UDP de input do server.
    pub udp_port: u16,
    /// Chave da sessão UDP (32 bytes). Segredo.
    pub udp_key: Vec<u8>,
    /// Token do dispositivo, só no primeiro pareamento (32 bytes). Segredo: guardar cifrado.
    pub device_token: Option<Vec<u8>>,
    /// UUID do server.
    pub server_id: String,
    /// Nome do server.
    pub server_name: String,
    /// Slot do controle (0–3).
    pub slot: u8,
    /// Taxa máxima de envio de input, em Hz.
    pub send_rate_hz: u16,
    /// O server quer pacotes com motion.
    pub gyro_enabled: bool,
    /// O server vai mandar rumble.
    pub rumble_enabled: bool,
    /// Sugestão de vibração local ao tocar.
    pub touch_haptics: bool,
    /// Fingerprint (SHA-256 do DER) do certificado visto, 32 bytes: é o que se fixa ao parear.
    pub fingerprint: Vec<u8>,
}

impl std::fmt::Debug for WelcomeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WelcomeInfo")
            .field("session_id", &self.session_id)
            .field("server_name", &self.server_name)
            .field("slot", &self.slot)
            .finish_non_exhaustive()
    }
}

/// Configuração do controle enviada pelo server (no `welcome` e a qualquer momento depois).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct ConfigInfo {
    /// Slot do controle (0–3).
    pub slot: u8,
    /// Taxa máxima de envio de input, em Hz.
    pub send_rate_hz: u16,
    /// O server quer pacotes com motion.
    pub gyro_enabled: bool,
    /// O server vai mandar rumble.
    pub rumble_enabled: bool,
    /// Sugestão de vibração local ao tocar.
    pub touch_haptics: bool,
}

/// Quem acompanha a sessão de controle depois do `welcome`. Implementado pelo Kotlin/Swift.
///
/// Os métodos rodam na thread do keepalive: não bloqueie neles.
#[uniffi::export(with_foreign)]
pub trait SessionListener: Send + Sync {
    /// O server mandou uma configuração nova (o usuário ligou o gyro, trocou o slot…).
    fn on_config(&self, config: ConfigInfo);

    /// A sessão terminou. `reason`: `local` (o app encerrou), `network` (a conexão caiu),
    /// `timeout` (15 s sem resposta) ou `bye:<motivo>` (o server encerrou: `kicked`, `revoked`,
    /// `shutdown`…). Chamado uma única vez.
    fn on_closed(&self, reason: String);
}

type Stream = StreamOwned<ClientConnection, TcpStream>;

/// Estado compartilhado entre quem chama e a thread do keepalive.
struct Shared {
    stream: Mutex<Stream>,
    stop: AtomicBool,
    started: AtomicBool,
}

impl Shared {
    fn stream(&self) -> MutexGuard<'_, Stream> {
        self.stream.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Conexão de controle aberta, depois do `welcome`.
#[derive(uniffi::Object)]
pub struct ControlConnection {
    welcome: WelcomeInfo,
    shared: Arc<Shared>,
}

#[uniffi::export]
impl ControlConnection {
    /// Conecta a `host:port`, faz o TLS e o `hello`, e espera o `welcome`.
    ///
    /// Erros: [`CoreError::Rejected`] (o server recusou, com o motivo), [`CoreError::Network`] e
    /// [`CoreError::FingerprintMismatch`] (o certificado não é o esperado).
    #[uniffi::constructor]
    pub fn connect(
        host: &str,
        port: u16,
        trust: TrustPolicy,
        device: DeviceInfo,
        credential: Credential,
    ) -> Result<Arc<Self>, CoreError> {
        let ip: IpAddr = host.parse().map_err(|_| CoreError::InvalidArgument {
            reason: format!("endereço inválido: {host}"),
        })?;
        let hello = Hello {
            proto_ver: vynpad_protocol::VERSION,
            device_id: Uuid::parse(&device.device_id).map_err(invalid)?,
            name: device.name,
            model: device.model,
            os_ver: device.os_ver,
            app_ver: Some(device.app_ver),
            caps: Caps {
                gyro: device.gyro,
                rumble: device.rumble,
            },
            auth: credential.into_auth()?,
        };
        let rule = Rule::from_policy(trust)?;

        let seen = Arc::new(Mutex::new(None));
        let mismatch = Arc::new(AtomicBool::new(false));
        let verifier = Arc::new(Verifier {
            rule,
            seen: Arc::clone(&seen),
            mismatch: Arc::clone(&mismatch),
            algs: ring::default_provider().signature_verification_algorithms,
        });
        let config = ClientConfig::builder_with_provider(Arc::new(ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(network)?
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_no_client_auth();
        let name = ServerName::try_from(SERVER_NAME).map_err(network)?;
        let connection = ClientConnection::new(Arc::new(config), name).map_err(network)?;

        let tcp = TcpStream::connect_timeout(&SocketAddr::new(ip, port), CONNECT_TIMEOUT)
            .map_err(network)?;
        tcp.set_nodelay(true).map_err(network)?;
        tcp.set_read_timeout(Some(IO_TIMEOUT)).map_err(network)?;
        tcp.set_write_timeout(Some(IO_TIMEOUT)).map_err(network)?;
        let mut stream = StreamOwned::new(connection, tcp);

        let outcome = exchange(&mut stream, &hello);
        // Um certificado errado derruba o handshake: o erro útil é esse, não o de I/O.
        if mismatch.load(Ordering::SeqCst) {
            return Err(CoreError::FingerprintMismatch);
        }
        let welcome = outcome?;

        let fingerprint = seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .ok_or_else(|| CoreError::Network {
                detail: "o TLS terminou sem mostrar o certificado".to_owned(),
            })?;
        Ok(Arc::new(Self {
            welcome: WelcomeInfo::from_welcome(welcome, fingerprint),
            shared: Arc::new(Shared {
                stream: Mutex::new(stream),
                stop: AtomicBool::new(false),
                started: AtomicBool::new(false),
            }),
        }))
    }

    /// O `welcome` recebido.
    pub fn welcome(&self) -> WelcomeInfo {
        self.welcome.clone()
    }

    /// O endereço IP do server com que esta conexão está falando (o destino do UDP de input).
    pub fn peer_host(&self) -> String {
        self.shared
            .stream()
            .sock
            .peer_addr()
            .map(|addr| addr.ip().to_string())
            .unwrap_or_default()
    }

    /// Mantém a sessão viva: responde aos `ping` do server na hora (ele manda um a cada 250 ms),
    /// manda o seu a cada 5 s de silêncio e avisa o [`SessionListener`] das mudanças. Sem isso o
    /// server derruba a conexão em 15 s. Roda numa thread própria; chame uma vez.
    // O UniFFI entrega o listener do Kotlin/Swift como `Arc` por valor.
    #[allow(clippy::needless_pass_by_value)]
    pub fn start(&self, listener: Arc<dyn SessionListener>) {
        if self.shared.started.swap(true, Ordering::SeqCst) {
            return;
        }
        let shared = Arc::clone(&self.shared);
        let spawned = std::thread::Builder::new()
            .name("vynpad-control".to_owned())
            .spawn({
                let listener = Arc::clone(&listener);
                move || {
                    let reason = keepalive(&shared, listener.as_ref());
                    listener.on_closed(reason);
                }
            });
        if spawned.is_err() {
            listener.on_closed("network".to_owned());
        }
    }

    /// Encerra a sessão com um `bye` limpo (libera o slot no server).
    pub fn disconnect(&self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        let mut stream = self.shared.stream();
        let bye = ControlMessage::Bye(Bye {
            reason: ByeReason::User,
            message: None,
        });
        // Se a conexão já caiu, não há mais nada a fazer.
        let _ = write_message(&mut *stream, &bye);
        stream.conn.send_close_notify();
        let _ = stream.flush();
        let _ = stream.sock.shutdown(std::net::Shutdown::Both);
    }
}

/// Quanto cada leitura espera antes de olhar o cancelamento e o relógio do keepalive.
const POLL: Duration = Duration::from_millis(100);
/// Silêncio de quem envia antes de mandar o próprio `ping`.
const PING_AFTER: Duration = Duration::from_secs(5);
/// Silêncio de quem recebe antes de dar a conexão por morta.
const DEAD_AFTER: Duration = Duration::from_secs(15);

/// O laço da sessão. Devolve o motivo do fim (ver [`SessionListener::on_closed`]).
fn keepalive(shared: &Shared, listener: &dyn SessionListener) -> String {
    let mut inbox: Vec<u8> = Vec::new();
    let mut last_rx = Instant::now();
    let mut last_tx = Instant::now();
    let mut chunk = [0u8; 2048];
    loop {
        if shared.stop.load(Ordering::SeqCst) {
            return "local".to_owned();
        }
        let mut configs: Vec<ConfigInfo> = Vec::new();
        let mut ended: Option<String> = None;
        {
            let mut stream = shared.stream();
            let _ = stream.sock.set_read_timeout(Some(POLL));
            match stream.read(&mut chunk) {
                Ok(0) => ended = Some("network".to_owned()),
                Ok(n) => {
                    inbox.extend_from_slice(&chunk[..n]);
                    last_rx = Instant::now();
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(_) => ended = Some("network".to_owned()),
            }
            while ended.is_none() {
                match next_frame(&mut inbox) {
                    Ok(Some(Incoming::Message(message))) => match message {
                        ControlMessage::Ping => {
                            if write_message(&mut *stream, &ControlMessage::Pong).is_err() {
                                ended = Some("network".to_owned());
                            }
                            last_tx = Instant::now();
                        }
                        ControlMessage::Config(c) => configs.push(ConfigInfo {
                            slot: c.slot,
                            send_rate_hz: c.send_rate_hz,
                            gyro_enabled: c.gyro_enabled,
                            rumble_enabled: c.rumble_enabled,
                            touch_haptics: c.touch_haptics,
                        }),
                        ControlMessage::Bye(bye) => {
                            ended = Some(format!("bye:{}", bye_code(bye.reason)));
                        }
                        // `pong`, `welcome` repetido, `reject` fora de hora: nada a fazer.
                        _ => {}
                    },
                    Ok(Some(Incoming::Ignored)) => {}
                    Ok(None) => break,
                    Err(_) => ended = Some("network".to_owned()),
                }
            }
            if ended.is_none() && last_tx.elapsed() >= PING_AFTER {
                if write_message(&mut *stream, &ControlMessage::Ping).is_err() {
                    ended = Some("network".to_owned());
                }
                last_tx = Instant::now();
            }
            if ended.is_none() && last_rx.elapsed() >= DEAD_AFTER {
                ended = Some("timeout".to_owned());
            }
        }
        // O lock já foi solto: o listener pode chamar `disconnect` sem travar.
        for config in configs {
            listener.on_config(config);
        }
        if let Some(reason) = ended {
            return reason;
        }
    }
}

/// Tira o próximo frame completo do buffer, se houver.
fn next_frame(inbox: &mut Vec<u8>) -> Result<Option<Incoming>, CoreError> {
    let Some(prefix) = inbox.first_chunk::<4>() else {
        return Ok(None);
    };
    let len = frame_len(*prefix).map_err(network)?;
    if inbox.len() < 4 + len {
        return Ok(None);
    }
    let incoming = decode_body(&inbox[4..4 + len]).map_err(network)?;
    inbox.drain(..4 + len);
    Ok(Some(incoming))
}

/// Código estável do motivo de um `bye` do server.
fn bye_code(reason: ByeReason) -> &'static str {
    match reason {
        ByeReason::User => "user",
        ByeReason::Shutdown => "shutdown",
        ByeReason::Revoked => "revoked",
        ByeReason::Kicked => "kicked",
        ByeReason::Timeout => "timeout",
        ByeReason::SeqExhausted => "seq_exhausted",
        ByeReason::Error => "error",
        ByeReason::Unknown => "unknown",
    }
}

impl Credential {
    fn into_auth(self) -> Result<Auth, CoreError> {
        Ok(match self {
            Self::PairingToken { token } => Auth::PairingToken {
                token: Secret::new(Bytes(fixed::<16>(token, "token de pareamento")?)),
            },
            Self::Pin { pin } => Auth::Pin {
                pin: Secret::new(pin),
            },
            Self::DeviceToken { token } => Auth::DeviceToken {
                token: Secret::new(Bytes(fixed::<32>(token, "token do dispositivo")?)),
            },
        })
    }
}

impl WelcomeInfo {
    fn from_welcome(w: vynpad_protocol::control::Welcome, fingerprint: [u8; 32]) -> Self {
        Self {
            session_id: w.session_id,
            udp_port: w.udp_port,
            udp_key: w.udp_key.expose().0.to_vec(),
            device_token: w.device_token.map(|t| t.expose().0.to_vec()),
            server_id: w.server_id.as_str().to_owned(),
            server_name: w.server_name,
            slot: w.config.slot,
            send_rate_hz: w.config.send_rate_hz,
            gyro_enabled: w.config.gyro_enabled,
            rumble_enabled: w.config.rumble_enabled,
            touch_haptics: w.config.touch_haptics,
            fingerprint: fingerprint.to_vec(),
        }
    }
}

fn fixed<const N: usize>(bytes: Vec<u8>, what: &str) -> Result<[u8; N], CoreError> {
    bytes
        .try_into()
        .map_err(|b: Vec<u8>| CoreError::InvalidArgument {
            reason: format!("{what} deve ter {N} bytes, recebeu {}", b.len()),
        })
}

fn invalid(e: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidArgument {
        reason: e.to_string(),
    }
}

fn network(e: impl std::fmt::Display) -> CoreError {
    CoreError::Network {
        detail: e.to_string(),
    }
}

/// `hello` e a espera pelo `welcome` (ou `reject`).
fn exchange(
    stream: &mut Stream,
    hello: &Hello,
) -> Result<vynpad_protocol::control::Welcome, CoreError> {
    write_message(stream, &ControlMessage::Hello(hello.clone()))?;
    loop {
        match read_message(stream)? {
            Incoming::Message(ControlMessage::Welcome(welcome)) => return Ok(welcome),
            Incoming::Message(ControlMessage::Reject(reject)) => {
                return Err(CoreError::Rejected {
                    reason: reject_code(reject.reason).to_owned(),
                    retry_after_s: reject.retry_after_s,
                });
            }
            Incoming::Message(other) => {
                return Err(network(format!("o server respondeu {other:?} ao hello")));
            }
            Incoming::Ignored => {}
        }
    }
}

/// Código estável do motivo da recusa (os mesmos de `reject.reason` no protocolo).
fn reject_code(reason: RejectReason) -> &'static str {
    match reason {
        RejectReason::BadRequest => "bad_request",
        RejectReason::BadAuth => "bad_auth",
        RejectReason::Locked => "locked",
        RejectReason::Revoked => "revoked",
        RejectReason::NoSlot => "no_slot",
        RejectReason::Version => "version",
        // Motivo que esta versão não conhece: erro genérico, como manda o protocolo.
        RejectReason::Unknown => "unknown",
    }
}

fn write_message(stream: &mut impl Write, message: &ControlMessage) -> Result<(), CoreError> {
    let frame = encode_frame(message).map_err(invalid)?;
    stream.write_all(&frame).map_err(network)?;
    stream.flush().map_err(network)
}

fn read_message(stream: &mut impl Read) -> Result<Incoming, CoreError> {
    let mut prefix = [0u8; 4];
    stream.read_exact(&mut prefix).map_err(network)?;
    let len = frame_len(prefix).map_err(network)?;
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).map_err(network)?;
    decode_body(&body).map_err(network)
}

/// Regra de confiança já validada.
#[derive(Debug, Clone, Copy)]
enum Rule {
    Pinned([u8; 32]),
    Prefix([u8; SHORT_FP_LEN]),
    FirstUse,
}

impl Rule {
    fn from_policy(policy: TrustPolicy) -> Result<Self, CoreError> {
        Ok(match policy {
            TrustPolicy::Pinned { fingerprint } => {
                Self::Pinned(fixed::<32>(fingerprint, "fingerprint")?)
            }
            TrustPolicy::ShortFingerprint { hex } => Self::Prefix(parse_short_hex(&hex)?),
            TrustPolicy::FirstUse => Self::FirstUse,
        })
    }

    fn accepts(self, fingerprint: &[u8; 32]) -> bool {
        match self {
            Self::Pinned(expected) => &expected == fingerprint,
            Self::Prefix(prefix) => fingerprint[..SHORT_FP_LEN] == prefix,
            Self::FirstUse => true,
        }
    }
}

fn parse_short_hex(hex: &str) -> Result<[u8; SHORT_FP_LEN], CoreError> {
    let bad = || CoreError::InvalidArgument {
        reason: format!(
            "o fingerprint curto deve ter {} caracteres hex",
            SHORT_FP_LEN * 2
        ),
    };
    if hex.len() != SHORT_FP_LEN * 2 || !hex.is_ascii() {
        return Err(bad());
    }
    let mut out = [0u8; SHORT_FP_LEN];
    for (byte, pair) in out.iter_mut().zip(hex.as_bytes().chunks(2)) {
        let pair = std::str::from_utf8(pair).map_err(|_| bad())?;
        *byte = u8::from_str_radix(pair, 16).map_err(|_| bad())?;
    }
    Ok(out)
}

/// Confia no certificado pelo fingerprint, mas confere de verdade a assinatura do handshake.
#[derive(Debug)]
struct Verifier {
    rule: Rule,
    seen: Arc<Mutex<Option<[u8; 32]>>>,
    mismatch: Arc<AtomicBool>,
    algs: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for Verifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let fingerprint: [u8; 32] = Sha256::digest(end_entity.as_ref()).into();
        if !self.rule.accepts(&fingerprint) {
            self.mismatch.store(true, Ordering::SeqCst);
            return Err(rustls::Error::General(
                "o fingerprint do certificado não é o esperado".into(),
            ));
        }
        *self.seen.lock().unwrap_or_else(PoisonError::into_inner) = Some(fingerprint);
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.algs)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.algs)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algs.supported_schemes()
    }
}
