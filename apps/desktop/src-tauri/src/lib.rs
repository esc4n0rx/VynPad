//! Shell Tauri do server: janela, plugins e commands expostos à UI.

use serde::Serialize;
use tauri::Manager;

const MAIN_WINDOW: &str = "main";

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
    let builder = tauri::Builder::default();

    // O single-instance precisa ser o primeiro plugin registrado.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        focus_main_window(app);
    }));

    builder
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![app_info])
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o PadGaming");
}
