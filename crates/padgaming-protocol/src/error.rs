//! Erros do protocolo UDP.

use core::fmt;

/// Motivo da recusa de um pacote UDP ou de uma operação de envio.
///
/// Os dez primeiros casos seguem a ordem normativa de validação da PROTOCOLO §6.5;
/// [`ProtoError::code`] devolve os mesmos códigos usados nos test vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtoError {
    /// Datagrama menor que o cabeçalho (12 bytes).
    TooShort,
    /// `magic` diferente de `0x4750`.
    BadMagic,
    /// `ver` diferente de [`crate::VERSION`].
    BadVersion,
    /// `type` desconhecido nesta versão.
    UnknownType,
    /// `type` válido, mas de uma direção que este lado não recebe.
    WrongDirection,
    /// Tamanho diferente do exato para o `type`.
    BadLength,
    /// `session_id` não corresponde a nenhuma sessão ativa.
    UnknownSession,
    /// `seq` menor ou igual ao último aceito (replay ou fora de ordem).
    StaleSeq,
    /// Tag AEAD inválida (pacote adulterado, chave ou nonce errados).
    BadTag,
    /// Payload autêntico, mas semanticamente inválido (bit de motion incoerente com o `type`).
    BadPayload,
    /// O buffer de saída não comporta o pacote.
    BufferTooSmall,
    /// O `seq` de envio chegou ao fim; a sessão precisa ser renovada (PROTOCOLO §6.3).
    SeqExhausted,
}

impl ProtoError {
    /// Código estável do erro, igual ao usado em `protocol/test-vectors/udp.json`.
    pub const fn code(self) -> &'static str {
        match self {
            Self::TooShort => "too_short",
            Self::BadMagic => "bad_magic",
            Self::BadVersion => "bad_version",
            Self::UnknownType => "unknown_type",
            Self::WrongDirection => "wrong_direction",
            Self::BadLength => "bad_length",
            Self::UnknownSession => "unknown_session",
            Self::StaleSeq => "stale_seq",
            Self::BadTag => "bad_tag",
            Self::BadPayload => "bad_payload",
            Self::BufferTooSmall => "buffer_too_small",
            Self::SeqExhausted => "seq_exhausted",
        }
    }
}

impl fmt::Display for ProtoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl core::error::Error for ProtoError {}
