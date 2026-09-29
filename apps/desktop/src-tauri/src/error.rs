//! Erro padronizado devolvido pelos commands ao frontend.
//!
//! O React recebe `{ code, message_key, details? }`: `code` para decidir o que fazer,
//! `message_key` para buscar o texto traduzido no i18n e `details` só para diagnóstico.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Internal,
    InvalidInput,
    /// Recurso indisponível nesta plataforma (só é devolvido fora do Windows).
    #[cfg_attr(windows, allow(dead_code))]
    Unsupported,
}

impl ErrorCode {
    fn message_key(self) -> &'static str {
        match self {
            Self::Internal => "errors.internal",
            Self::InvalidInput => "errors.invalid_input",
            Self::Unsupported => "errors.unsupported",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AppError {
    pub code: ErrorCode,
    pub message_key: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, details: Option<String>) -> Self {
        Self {
            code,
            message_key: code.message_key(),
            details,
        }
    }

    pub fn invalid_input(details: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, Some(details.into()))
    }
}

/// Qualquer erro inesperado vira `internal`, e é registrado no log com a causa completa.
///
/// `AppError` não implementa `std::error::Error` de propósito, para esta conversão genérica
/// não conflitar com `From<AppError> for AppError`.
impl<E: std::error::Error> From<E> for AppError {
    fn from(err: E) -> Self {
        tracing::error!(error = %err, "erro interno em command");
        Self::new(ErrorCode::Internal, Some(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializa_no_formato_esperado_pelo_frontend() {
        let json = serde_json::to_value(AppError::invalid_input("muito longo")).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "code": "invalid_input",
                "message_key": "errors.invalid_input",
                "details": "muito longo",
            })
        );
    }

    #[test]
    fn omite_details_quando_ausente() {
        let json = serde_json::to_value(AppError::new(ErrorCode::Internal, None)).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "internal", "message_key": "errors.internal" })
        );
    }

    #[test]
    fn erro_generico_vira_internal() {
        let err: AppError = std::io::Error::other("disco cheio").into();
        assert_eq!(err.code, ErrorCode::Internal);
        assert_eq!(err.details.as_deref(), Some("disco cheio"));
    }
}
