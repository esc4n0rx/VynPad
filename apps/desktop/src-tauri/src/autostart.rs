//! Iniciar com o Windows (T-607): liga e desliga a entrada do registro (`Run`) conforme a
//! configuração. O app registrado abre normalmente; quem decide se a janela aparece é
//! "Iniciar minimizado" (`shell::should_start_minimized`).

use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::ManagerExt;

/// Põe o autostart do sistema de acordo com `enabled` (sem mexer se já estiver).
pub fn apply<R: Runtime>(app: &AppHandle<R>, enabled: bool) {
    let launch = app.autolaunch();
    let current = launch.is_enabled().unwrap_or(false);
    if current == enabled {
        return;
    }
    let result = if enabled {
        launch.enable()
    } else {
        launch.disable()
    };
    match result {
        Ok(()) => tracing::info!(enabled, "iniciar com o sistema atualizado"),
        Err(error) => tracing::warn!(%error, enabled, "não consegui mudar o iniciar com o sistema"),
    }
}
