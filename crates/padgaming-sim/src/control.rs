//! O canal de controle do client simulado (PROTOCOLO §5): TLS 1.3 com fingerprint fixado,
//! `hello` e `welcome`. Bloqueante, como o resto do simulador.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, bail};
use padgaming_protocol::control::{
    Auth, Caps, ControlMessage, Hello, Incoming, Reject, Uuid, Welcome, decode_body, encode_frame,
    frame_len,
};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{WebPkiSupportedAlgorithms, ring};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, ClientConnection, DigitallySignedStruct, SignatureScheme, StreamOwned};
use sha2::{Digest, Sha256};

/// Nome usado no SNI; o certificado do server é confiado pelo fingerprint, não pelo nome.
const SERVER_NAME: &str = "padgaming.local";
/// Prazo de leitura e escrita no canal de controle.
const IO_TIMEOUT: Duration = Duration::from_secs(10);

/// Em quem confiar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trust {
    /// Só o certificado com este fingerprint (SHA-256 do DER): o pinning de quem já pareou.
    Pinned([u8; 32]),
    /// Confia no primeiro certificado (pareamento por PIN); o fingerprint visto fica em
    /// [`ControlSession::fingerprint`] para o usuário conferir.
    FirstUse,
}

/// O que o client anuncia no `hello`.
#[derive(Debug, Clone)]
pub struct Identity {
    /// UUID do aparelho simulado.
    pub device_id: Uuid,
    /// Nome exibido no server.
    pub name: String,
    /// Recursos.
    pub caps: Caps,
    /// Versão do protocolo anunciada.
    pub proto_ver: u8,
}

impl Identity {
    /// Um aparelho simulado com nome `PadGaming Sim`, com gyro e vibração.
    pub fn simulated(device_id: Uuid) -> Self {
        Self {
            device_id,
            name: "PadGaming Sim".to_owned(),
            caps: Caps {
                gyro: true,
                rumble: true,
            },
            proto_ver: 1,
        }
    }

    fn hello(&self, auth: Auth) -> Hello {
        Hello {
            proto_ver: self.proto_ver,
            device_id: self.device_id.clone(),
            name: self.name.clone(),
            model: "padgaming-sim".to_owned(),
            os_ver: std::env::consts::OS.to_owned(),
            app_ver: Some(env!("CARGO_PKG_VERSION").to_owned()),
            caps: self.caps,
            auth,
        }
    }
}

/// Resultado do handshake.
pub enum Outcome {
    /// Sessão criada.
    Welcome(Box<ControlSession>),
    /// O server recusou.
    Rejected(Reject),
}

/// Conexão de controle aberta, depois do `welcome`.
pub struct ControlSession {
    /// O `welcome` recebido (chave UDP, slot, `device_token`...).
    pub welcome: Welcome,
    /// Fingerprint do certificado visto (SHA-256 do DER).
    pub fingerprint: [u8; 32],
    stream: StreamOwned<ClientConnection, TcpStream>,
}

impl ControlSession {
    /// Envia uma mensagem de controle.
    pub fn send(&mut self, message: &ControlMessage) -> anyhow::Result<()> {
        write_message(&mut self.stream, message)
    }

    /// Espera a próxima mensagem conhecida (ignora as de `type` desconhecido).
    pub fn recv(&mut self, timeout: Duration) -> anyhow::Result<ControlMessage> {
        self.stream.sock.set_read_timeout(Some(timeout))?;
        loop {
            if let Incoming::Message(message) = read_message(&mut self.stream)? {
                return Ok(message);
            }
        }
    }

    /// Fecha a conexão TCP de uma vez, sem `bye` (simula a queda da rede).
    pub fn drop_connection(self) {
        let _ = self.stream.sock.shutdown(std::net::Shutdown::Both);
    }
}

/// Conecta, faz o TLS e o `hello`, e espera o `welcome` ou o `reject`.
pub fn connect(
    server: SocketAddr,
    trust: Trust,
    identity: &Identity,
    auth: Auth,
) -> anyhow::Result<Outcome> {
    let seen = Arc::new(Mutex::new(None));
    let verifier = Arc::new(Verifier {
        trust,
        seen: Arc::clone(&seen),
        algs: ring::default_provider().signature_verification_algorithms,
    });
    let config = ClientConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    let name = ServerName::try_from(SERVER_NAME)?;
    let connection = ClientConnection::new(Arc::new(config), name)?;

    let tcp = TcpStream::connect_timeout(&server, IO_TIMEOUT)
        .with_context(|| format!("não consegui conectar a {server}"))?;
    tcp.set_nodelay(true)?;
    tcp.set_read_timeout(Some(IO_TIMEOUT))?;
    tcp.set_write_timeout(Some(IO_TIMEOUT))?;
    let mut stream = StreamOwned::new(connection, tcp);

    write_message(&mut stream, &ControlMessage::Hello(identity.hello(auth)))?;
    let fingerprint = seen
        .lock()
        .map_err(|_| anyhow::anyhow!("estado do verificador envenenado"))?
        .ok_or_else(|| anyhow::anyhow!("o TLS terminou sem mostrar o certificado"))?;

    loop {
        match read_message(&mut stream)? {
            Incoming::Message(ControlMessage::Welcome(welcome)) => {
                return Ok(Outcome::Welcome(Box::new(ControlSession {
                    welcome,
                    fingerprint,
                    stream,
                })));
            }
            Incoming::Message(ControlMessage::Reject(reject)) => {
                return Ok(Outcome::Rejected(reject));
            }
            Incoming::Message(other) => bail!("o server respondeu {other:?} ao hello"),
            Incoming::Ignored => {}
        }
    }
}

fn write_message(stream: &mut impl Write, message: &ControlMessage) -> anyhow::Result<()> {
    let frame = encode_frame(message).map_err(|e| anyhow::anyhow!("mensagem inválida: {e}"))?;
    stream.write_all(&frame)?;
    stream.flush()?;
    Ok(())
}

fn read_message(stream: &mut impl Read) -> anyhow::Result<Incoming> {
    let mut prefix = [0u8; 4];
    stream.read_exact(&mut prefix)?;
    let len = frame_len(prefix).map_err(|e| anyhow::anyhow!("frame inválido: {e}"))?;
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body)?;
    decode_body(&body).map_err(|e| anyhow::anyhow!("mensagem inválida: {e}"))
}

/// Confia no certificado pelo fingerprint, mas confere de verdade a assinatura do handshake.
#[derive(Debug)]
struct Verifier {
    trust: Trust,
    seen: Arc<Mutex<Option<[u8; 32]>>>,
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
        if let Trust::Pinned(expected) = self.trust
            && expected != fingerprint
        {
            return Err(rustls::Error::General(
                "o fingerprint do certificado não é o fixado no pareamento".into(),
            ));
        }
        *self
            .seen
            .lock()
            .map_err(|_| rustls::Error::General("estado envenenado".into()))? = Some(fingerprint);
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
