//! Erro padronizado devolvido pelos commands ao frontend.
//!
//! O React recebe `{ code, message_key, details?, in_use? }`: `code` para decidir o que fazer,
//! `message_key` para buscar o texto traduzido no i18n, `details` com o detalhe (validações vêm
//! aqui, já em português, para mostrar sob a mensagem) e `in_use` quando apagar um perfil em uso
//! pede confirmação.

use serde::Serialize;
use specta::Type;
use vynpad_server::CoreError;
use vynpad_storage::DataError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Internal,
    InvalidInput,
    /// Recurso indisponível nesta plataforma (só é devolvido fora do Windows).
    #[cfg_attr(windows, allow(dead_code))]
    Unsupported,
    /// O registro pedido não existe (dispositivo, perfil, slot).
    NotFound,
    /// O perfil padrão do app não pode ser editado nem apagado.
    BuiltinProfile,
    /// O perfil está em uso; apagar exige confirmação (`in_use` traz os números).
    ProfileInUse,
    /// O arquivo `.padprofile` é inválido.
    ImportInvalid,
    /// Sem rede local para o QR code.
    NoNetwork,
    /// O usuário recusou o pedido de permissão do Windows (UAC).
    Cancelled,
    /// Não deu para procurar, baixar ou instalar a atualização (T-704).
    UpdateFailed,
}

impl ErrorCode {
    const fn message_key(self) -> &'static str {
        match self {
            Self::Internal => "errors.internal",
            Self::InvalidInput => "errors.invalid_input",
            Self::Unsupported => "errors.unsupported",
            Self::NotFound => "errors.not_found",
            Self::BuiltinProfile => "errors.builtin_profile",
            Self::ProfileInUse => "errors.profile_in_use",
            Self::ImportInvalid => "errors.import_invalid",
            Self::NoNetwork => "errors.no_network",
            Self::Cancelled => "errors.cancelled",
            Self::UpdateFailed => "errors.update_failed",
        }
    }
}

/// Quanto o perfil está em uso.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct InUse {
    pub devices: u32,
    pub slots: u32,
}

#[derive(Debug, Serialize, Type)]
pub struct AppError {
    pub code: ErrorCode,
    pub message_key: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_use: Option<InUse>,
}

impl AppError {
    pub fn new(code: ErrorCode, details: Option<String>) -> Self {
        Self {
            code,
            message_key: code.message_key(),
            details,
            in_use: None,
        }
    }

    pub fn invalid_input(details: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, Some(details.into()))
    }

    pub fn not_found(what: &str) -> Self {
        Self::new(ErrorCode::NotFound, Some(what.to_owned()))
    }

    /// Erro inesperado: vira `internal` e a causa completa vai para o log.
    pub fn internal(error: &dyn std::error::Error) -> Self {
        tracing::error!(%error, "erro interno em command");
        Self::new(ErrorCode::Internal, Some(error.to_string()))
    }
}

impl From<DataError> for AppError {
    fn from(error: DataError) -> Self {
        match error {
            DataError::NotFound(what) => Self::not_found(what),
            DataError::Invalid(why) => Self::invalid_input(why),
            DataError::Builtin => Self::new(ErrorCode::BuiltinProfile, None),
            DataError::InUse { devices, slots } => Self {
                in_use: Some(InUse { devices, slots }),
                ..Self::new(ErrorCode::ProfileInUse, None)
            },
            DataError::Import(why) => Self::new(ErrorCode::ImportInvalid, Some(why)),
            DataError::Database(_) => Self::internal(&error),
        }
    }
}

impl From<CoreError> for AppError {
    fn from(error: CoreError) -> Self {
        match error {
            CoreError::Data(data) => data.into(),
            CoreError::NoNetwork => Self::new(ErrorCode::NoNetwork, None),
            other => Self::internal(&other),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::internal(&error)
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
    fn erros_de_dados_viram_codigos_estaveis() {
        let code = |e: DataError| AppError::from(e).code;
        assert_eq!(code(DataError::NotFound("perfil")), ErrorCode::NotFound);
        assert_eq!(
            code(DataError::Invalid("x".into())),
            ErrorCode::InvalidInput
        );
        assert_eq!(code(DataError::Builtin), ErrorCode::BuiltinProfile);
        assert_eq!(
            code(DataError::Import("x".into())),
            ErrorCode::ImportInvalid
        );
        assert_eq!(
            AppError::from(CoreError::NoNetwork).code,
            ErrorCode::NoNetwork
        );
    }

    #[test]
    fn perfil_em_uso_traz_os_numeros_para_a_confirmacao() {
        let error = AppError::from(DataError::InUse {
            devices: 2,
            slots: 1,
        });
        assert_eq!(error.code, ErrorCode::ProfileInUse);
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(
            json["in_use"],
            serde_json::json!({ "devices": 2, "slots": 1 })
        );
    }
}
