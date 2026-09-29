//! Identidade do server: certificado autoassinado (ECDSA P-256), `server_id` e fingerprint.
//!
//! Os três arquivos ficam no diretório de dados do app (`server.crt`, `server.key`,
//! `server_id`). No Unix a chave é gravada com modo `0600`; no Windows ela herda a ACL do
//! diretório de dados do usuário (`%APPDATA%`), que só o próprio usuário e administradores leem.
//!
//! Regenerar a identidade ([`ServerIdentity::regenerate`]) muda o fingerprint e, com ele,
//! invalida todos os pareamentos: os clients fazem pinning do certificado.

use std::fmt;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use padgaming_protocol::control::Uuid;
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_ECDSA_P256_SHA256};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use sha2::{Digest, Sha256};

use crate::random_bytes;

const CERT_FILE: &str = "server.crt";
const KEY_FILE: &str = "server.key";
const ID_FILE: &str = "server_id";
/// Validade do certificado, em anos.
const VALIDITY_YEARS: i32 = 10;
/// Nome de servidor no certificado; os clients confiam no fingerprint, não no nome.
const CERT_DNS_NAME: &str = "padgaming.local";

/// Erro ao criar, ler ou usar a identidade do server.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// Falha de leitura/escrita em `path`.
    #[error("{}: {source}", path.display())]
    Io {
        /// Arquivo envolvido.
        path: PathBuf,
        /// Erro original.
        source: io::Error,
    },
    /// Falha ao gerar o certificado.
    #[error("falha ao gerar o certificado: {0}")]
    Generate(#[from] rcgen::Error),
    /// Arquivos ausentes ou ilegíveis. Não regeneramos sozinhos: isso invalidaria os pareamentos.
    #[error("identidade corrompida ({0}); use \"Regenerar identidade\" para recriá-la")]
    Corrupt(String),
    /// Configuração TLS recusada pelo rustls.
    #[error("configuração TLS inválida: {0}")]
    Tls(#[from] rustls::Error),
}

/// Certificado, chave privada e `server_id` do server.
#[derive(Clone)]
pub struct ServerIdentity {
    server_id: Uuid,
    cert: CertificateDer<'static>,
    key: Arc<PrivateKeyDer<'static>>,
    fingerprint: [u8; 32],
}

impl fmt::Debug for ServerIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerIdentity")
            .field("server_id", &self.server_id)
            .field("fingerprint", &self.fingerprint_b64())
            .finish_non_exhaustive()
    }
}

impl ServerIdentity {
    /// Gera uma identidade nova, só em memória.
    pub fn generate() -> Result<Self, IdentityError> {
        let (identity, _) = Self::generate_with_pem()?;
        Ok(identity)
    }

    fn generate_with_pem() -> Result<(Self, PemFiles), IdentityError> {
        let server_id = new_uuid_v4();
        let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;

        let mut params = CertificateParams::new(vec![CERT_DNS_NAME.to_owned()])?;
        params.distinguished_name = DistinguishedName::new();
        params.distinguished_name.push(
            DnType::CommonName,
            format!("PadGaming {}", server_id.as_str()),
        );
        // Margem de 1 ano para trás tolera relógios levemente desajustados.
        let year = current_year();
        params.not_before = rcgen::date_time_ymd(year - 1, 1, 1);
        params.not_after = rcgen::date_time_ymd(year + VALIDITY_YEARS, 12, 31);
        let cert = params.self_signed(&key_pair)?;

        let pem = PemFiles {
            cert: cert.pem(),
            key: key_pair.serialize_pem(),
        };
        let identity = Self::from_parts(server_id, cert.der().clone(), &pem.key)?;
        Ok((identity, pem))
    }

    fn from_parts(
        server_id: Uuid,
        cert: CertificateDer<'static>,
        key_pem: &str,
    ) -> Result<Self, IdentityError> {
        let key = PrivateKeyDer::from_pem_slice(key_pem.as_bytes())
            .map_err(|e| IdentityError::Corrupt(format!("chave privada: {e}")))?;
        let fingerprint = Sha256::digest(cert.as_ref()).into();
        Ok(Self {
            server_id,
            cert,
            key: Arc::new(key),
            fingerprint,
        })
    }

    /// Carrega a identidade de `dir`, ou cria uma nova no primeiro boot.
    ///
    /// Se só parte dos arquivos existir (ou eles não fizerem sentido), devolve
    /// [`IdentityError::Corrupt`] em vez de regenerar em silêncio.
    pub fn load_or_create(dir: &Path) -> Result<Self, IdentityError> {
        let paths = Paths::new(dir);
        let present = [&paths.cert, &paths.key, &paths.id]
            .iter()
            .filter(|p| p.exists())
            .count();
        match present {
            0 => Self::regenerate(dir),
            3 => Self::load(&paths),
            _ => Err(IdentityError::Corrupt(
                "faltam arquivos da identidade".to_owned(),
            )),
        }
    }

    /// Descarta a identidade de `dir` e cria outra. **Invalida todos os pareamentos.**
    pub fn regenerate(dir: &Path) -> Result<Self, IdentityError> {
        let paths = Paths::new(dir);
        fs::create_dir_all(dir).map_err(|e| io_err(dir, e))?;
        let (identity, pem) = Self::generate_with_pem()?;
        // Chave primeiro: se algo falhar no meio, o `load_or_create` acusa arquivos faltando.
        write_private(&paths.key, pem.key.as_bytes())?;
        write_private(&paths.cert, pem.cert.as_bytes())?;
        write_private(&paths.id, identity.server_id.as_str().as_bytes())?;
        Ok(identity)
    }

    fn load(paths: &Paths) -> Result<Self, IdentityError> {
        let read = |p: &Path| fs::read_to_string(p).map_err(|e| io_err(p, e));
        let id_text = read(&paths.id)?;
        let server_id = Uuid::parse(id_text.trim())
            .map_err(|_| IdentityError::Corrupt("server_id inválido".to_owned()))?;
        let cert_pem = read(&paths.cert)?;
        let cert = CertificateDer::from_pem_slice(cert_pem.as_bytes())
            .map_err(|e| IdentityError::Corrupt(format!("certificado: {e}")))?;
        Self::from_parts(server_id, cert, &read(&paths.key)?)
    }

    /// Identificador estável do server (UUID v4).
    pub fn server_id(&self) -> &Uuid {
        &self.server_id
    }

    /// Certificado em DER.
    pub fn cert_der(&self) -> &CertificateDer<'static> {
        &self.cert
    }

    /// SHA-256 do certificado em DER.
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    /// Fingerprint em base64url sem padding (43 caracteres), o formato do QR e do mDNS.
    pub fn fingerprint_b64(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.fingerprint)
    }

    /// Configuração TLS do canal de controle: **somente TLS 1.3**, sem autenticação de client
    /// (a autenticação é o pareamento/`device_token` da camada de aplicação).
    pub fn tls_config(&self) -> Result<Arc<rustls::ServerConfig>, IdentityError> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        // A versão é fixada aqui e não só pelas features do rustls, que outras crates do
        // workspace poderiam ligar (a unificação de features reativaria o TLS 1.2).
        let config = rustls::ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_no_client_auth()
            .with_single_cert(vec![self.cert.clone()], self.key.clone_key())?;
        Ok(Arc::new(config))
    }
}

struct PemFiles {
    cert: String,
    key: String,
}

struct Paths {
    cert: PathBuf,
    key: PathBuf,
    id: PathBuf,
}

impl Paths {
    fn new(dir: &Path) -> Self {
        Self {
            cert: dir.join(CERT_FILE),
            key: dir.join(KEY_FILE),
            id: dir.join(ID_FILE),
        }
    }
}

fn io_err(path: &Path, source: io::Error) -> IdentityError {
    IdentityError::Io {
        path: path.to_owned(),
        source,
    }
}

/// Grava o arquivo com permissões restritas (`0600` no Unix).
fn write_private(path: &Path, contents: &[u8]) -> Result<(), IdentityError> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|e| io_err(path, e))?;
    file.write_all(contents).map_err(|e| io_err(path, e))?;
    file.sync_all().map_err(|e| io_err(path, e))
}

/// UUID v4 em texto minúsculo, a partir do gerador do sistema.
fn new_uuid_v4() -> Uuid {
    let mut b = random_bytes::<16>();
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let mut text = String::with_capacity(36);
    for (i, byte) in b.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            text.push('-');
        }
        for nibble in [byte >> 4, byte & 0x0f] {
            text.push(char::from_digit(u32::from(nibble), 16).expect("nibble < 16"));
        }
    }
    Uuid::parse(&text).expect("UUID gerado é sempre válido")
}

/// Ano civil aproximado do relógio do sistema (suficiente para a validade do certificado).
fn current_year() -> i32 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    1970 + i32::try_from(secs / 31_556_952).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::RootCertStore;
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::{TcpListener, TcpStream};
    use tokio_rustls::{TlsAcceptor, TlsConnector};

    #[test]
    fn generates_identity_with_expected_shape() {
        let id = ServerIdentity::generate().unwrap();
        assert_eq!(id.fingerprint_b64().len(), 43);
        let uuid = id.server_id().as_str();
        assert_eq!(uuid.len(), 36);
        assert_eq!(&uuid[14..15], "4", "versão 4");
        assert_eq!(
            id.fingerprint(),
            <[u8; 32]>::from(Sha256::digest(id.cert_der()))
        );
    }

    #[test]
    fn persists_and_reloads_the_same_identity() {
        let dir = tempfile::tempdir().unwrap();
        let first = ServerIdentity::load_or_create(dir.path()).unwrap();
        let second = ServerIdentity::load_or_create(dir.path()).unwrap();
        assert_eq!(first.fingerprint(), second.fingerprint());
        assert_eq!(first.server_id(), second.server_id());
    }

    #[test]
    fn regenerate_changes_fingerprint_and_id() {
        let dir = tempfile::tempdir().unwrap();
        let old = ServerIdentity::load_or_create(dir.path()).unwrap();
        let new = ServerIdentity::regenerate(dir.path()).unwrap();
        assert_ne!(old.fingerprint(), new.fingerprint());
        assert_ne!(old.server_id(), new.server_id());
        let reloaded = ServerIdentity::load_or_create(dir.path()).unwrap();
        assert_eq!(reloaded.fingerprint(), new.fingerprint());
    }

    #[test]
    fn partial_or_garbled_files_are_an_error_not_a_silent_regeneration() {
        let dir = tempfile::tempdir().unwrap();
        ServerIdentity::load_or_create(dir.path()).unwrap();
        fs::remove_file(dir.path().join(KEY_FILE)).unwrap();
        assert!(matches!(
            ServerIdentity::load_or_create(dir.path()),
            Err(IdentityError::Corrupt(_))
        ));

        let dir = tempfile::tempdir().unwrap();
        ServerIdentity::load_or_create(dir.path()).unwrap();
        fs::write(dir.path().join(CERT_FILE), "lixo").unwrap();
        assert!(matches!(
            ServerIdentity::load_or_create(dir.path()),
            Err(IdentityError::Corrupt(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn private_key_is_owner_only() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().unwrap();
        ServerIdentity::load_or_create(dir.path()).unwrap();
        let mode = fs::metadata(dir.path().join(KEY_FILE))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn debug_does_not_print_the_key() {
        let id = ServerIdentity::generate().unwrap();
        let text = format!("{id:?}");
        assert!(!text.contains("PRIVATE"));
        assert!(text.contains("fingerprint"));
    }

    /// Handshake real em loopback: TLS 1.3 negociado e dados trafegando.
    #[tokio::test]
    async fn tls_handshake_negotiates_tls13() {
        let id = ServerIdentity::generate().unwrap();
        let acceptor = TlsAcceptor::from(id.tls_config().unwrap());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut tls = acceptor.accept(tcp).await.unwrap();
            let mut buf = [0u8; 4];
            tls.read_exact(&mut buf).await.unwrap();
            tls.write_all(&buf).await.unwrap();
            tls.shutdown().await.unwrap();
        });

        let mut roots = RootCertStore::empty();
        roots.add(id.cert_der().clone()).unwrap();
        let client_config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(client_config));

        let tcp = TcpStream::connect(addr).await.unwrap();
        let name = rustls::pki_types::ServerName::try_from(CERT_DNS_NAME).unwrap();
        let mut tls = connector.connect(name, tcp).await.unwrap();
        assert_eq!(
            tls.get_ref().1.protocol_version(),
            Some(rustls::ProtocolVersion::TLSv1_3)
        );
        tls.write_all(b"ping").await.unwrap();
        let mut echo = [0u8; 4];
        tls.read_exact(&mut echo).await.unwrap();
        assert_eq!(&echo, b"ping");
        server.await.unwrap();
    }
}
