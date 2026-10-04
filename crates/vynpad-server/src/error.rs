//! Erros do `Core`.

use vynpad_net::control::StartError;
use vynpad_net::discovery::DiscoveryError;
use vynpad_net::identity::IdentityError;
use vynpad_storage::{DataError, StorageError};

/// Falha ao iniciar ou operar o `Core`.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// Banco de dados (abrir, migrar).
    #[error(transparent)]
    Storage(#[from] StorageError),
    /// Operação de dados (configurações, perfis, dispositivos).
    #[error(transparent)]
    Data(#[from] DataError),
    /// Certificado do server.
    #[error("identidade do server: {0}")]
    Identity(#[from] IdentityError),
    /// Falha de I/O (pasta de dados, socket UDP).
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// Servidor de controle TLS.
    #[error(transparent)]
    Control(#[from] StartError),
    /// Descoberta (mDNS e broadcast).
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
    /// Não há interface de rede utilizável para mostrar o QR code.
    #[error("nenhuma rede local encontrada: conecte o PC ao Wi-Fi ou à rede cabeada")]
    NoNetwork,
    /// Não foi possível montar o QR code.
    #[error("não foi possível montar o QR code: {0}")]
    Pairing(String),
}
