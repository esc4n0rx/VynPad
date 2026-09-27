//! Shell Tauri do server: janela, plugins e commands expostos à UI.

mod error;
mod logging;

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::error::AppError;

const MAIN_WINDOW: &str = "main";

/// Limite por mensagem de log vinda da webview (evita inundar o arquivo).
const MAX_FRONTEND_LOG_BYTES: usize = 8 * 1024;

#[derive(Serialize)]
struct AppInfo {
    app_version: &'static str,
    core_version: &'static str,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        app_version: env!("CARGO_PKG_VERSION"),
        core_version: padgaming_core::VERSION,
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum FrontendLogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

/// Recebe logs do React e os grava no mesmo arquivo do backend, com target `frontend`.
#[tauri::command]
fn log_from_frontend(level: FrontendLogLevel, message: &str) -> Result<(), AppError> {
    if message.len() > MAX_FRONTEND_LOG_BYTES {
        return Err(AppError::invalid_input(format!(
            "mensagem de log com {} bytes (máximo {MAX_FRONTEND_LOG_BYTES})",
            message.len()
        )));
    }
    match level {
        FrontendLogLevel::Error => tracing::error!(target: "frontend", "{message}"),
        FrontendLogLevel::Warn => tracing::warn!(target: "frontend", "{message}"),
        FrontendLogLevel::Info => tracing::info!(target: "frontend", "{message}"),
        FrontendLogLevel::Debug => tracing::debug!(target: "frontend", "{message}"),
    }
    Ok(())
}

fn register_commands<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![app_info, log_from_frontend])
}

/// Traz a janela principal para frente (usado quando uma segunda instância é aberta).
fn focus_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init();
    tracing::info!(
        app = env!("CARGO_PKG_VERSION"),
        core = padgaming_core::VERSION,
        logs = ?logging::log_dir(),
        "PadGaming iniciando"
    );

    let builder = tauri::Builder::default();

    // O single-instance precisa ser o primeiro plugin registrado.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        tracing::info!("segunda instância aberta; focando a janela existente");
        focus_main_window(app);
    }));

    register_commands(builder)
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o PadGaming");
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use tauri::ipc::{CallbackFn, InvokeBody};
    use tauri::test::{INVOKE_KEY, get_ipc_response, mock_builder, mock_context, noop_assets};
    use tauri::webview::InvokeRequest;

    use super::*;

    /// Chama um command pela camada IPC do Tauri, como o frontend faria.
    fn invoke(cmd: &str, args: Value) -> Result<Value, Value> {
        let app = register_commands(mock_builder())
            .build(mock_context(noop_assets()))
            .expect("app de teste");
        let webview =
            tauri::WebviewWindowBuilder::new(&app, MAIN_WINDOW, tauri::WebviewUrl::default())
                .build()
                .expect("janela de teste");
        get_ipc_response(
            &webview,
            InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: "http://tauri.localhost".parse().unwrap(),
                body: InvokeBody::Json(args),
                headers: tauri::http::HeaderMap::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
        .map(|body| body.deserialize::<Value>().expect("resposta JSON"))
    }

    #[test]
    fn app_info_devolve_as_versoes() {
        let info = invoke("app_info", json!({})).unwrap();
        assert_eq!(info["app_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(info["core_version"], padgaming_core::VERSION);
    }

    #[test]
    fn log_do_frontend_valido_e_aceito() {
        let res = invoke(
            "log_from_frontend",
            json!({ "level": "info", "message": "olá" }),
        );
        assert_eq!(res, Ok(Value::Null));
    }

    #[test]
    fn erro_de_command_chega_com_code_e_message_key() {
        let message = "x".repeat(MAX_FRONTEND_LOG_BYTES + 1);
        let err = invoke(
            "log_from_frontend",
            json!({ "level": "info", "message": message }),
        )
        .unwrap_err();
        assert_eq!(err["code"], "invalid_input");
        assert_eq!(err["message_key"], "errors.invalid_input");
    }
}
