//! Erros que atravessam a fronteira com o Kotlin/Swift.

use vynpad_protocol::ProtoError;
use vynpad_protocol::pairing_uri::PairingUriError;

/// Erro de uma chamada ao núcleo. Os `code` são os estáveis dos test vectors, então a UI pode
/// decidir a mensagem sem depender de texto.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum CoreError {
    /// Pacote UDP recusado ou envio impossível (`stale_seq`, `bad_tag`, `seq_exhausted`…).
    #[error("protocolo: {code}")]
    Protocol {
        /// Código do [`ProtoError`].
        code: String,
    },
    /// URI de pareamento inválida (`bad_scheme`, `missing_param`, `bad_param`…).
    #[error("uri de pareamento inválida: {code}")]
    PairingUri {
        /// Código do [`PairingUriError`].
        code: String,
    },
    /// Resposta de descoberta malformada.
    #[error("resposta de descoberta inválida: {reason}")]
    Discovery {
        /// Descrição do problema.
        reason: String,
    },
    /// Argumento fora do contrato (por exemplo, chave que não tem 32 bytes).
    #[error("argumento inválido: {reason}")]
    InvalidArgument {
        /// Descrição do problema.
        reason: String,
    },
}

impl From<ProtoError> for CoreError {
    fn from(e: ProtoError) -> Self {
        Self::Protocol {
            code: e.code().to_owned(),
        }
    }
}

impl From<PairingUriError> for CoreError {
    fn from(e: PairingUriError) -> Self {
        Self::PairingUri {
            code: e.code().to_owned(),
        }
    }
}
