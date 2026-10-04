//! Ciclo de vida da janela principal (T-607): criar sob demanda, fechar para a bandeja, iniciar
//! minimizado e navegar a partir do menu da bandeja.
//!
//! A janela principal não existe até alguém pedir (clique na bandeja, segunda instância, ou a
//! abertura normal): fechar para a bandeja **destrói** a janela e libera a webview, em vez de só
//! escondê-la. Quem mantém o app vivo sem janela é o `RunEvent::ExitRequested` (ver `lib.rs`).

use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, Runtime, State, WebviewWindowBuilder, Window, WindowEvent};
use tauri_specta::Event;

use crate::state::AppState;

pub const MAIN_WINDOW: &str = "main";

/// Quanto a janela espera, na primeira vez que fecha para a bandeja, para o aviso ser lido.
const HINT_VISIBLE: Duration = Duration::from_millis(2200);

/// O aviso "o `PadGaming` continua rodando na bandeja" (`tray:hint`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct TrayHint {}

/// Há uma navegação pedida pela bandeja esperando a UI (`tray:navigate`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct TrayNavigate {}

/// Para onde a bandeja pediu para ir. Fica guardado porque a janela pode nem existir ainda quando
/// o menu é clicado: a UI lê ao abrir (`take_pending_navigation`) e também quando o evento chega.
#[derive(Default)]
pub struct PendingNav(Mutex<Option<String>>);

impl PendingNav {
    fn set(&self, path: &str) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(path.to_owned());
    }

    fn take(&self) -> Option<String> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).take()
    }
}

/// A UI pergunta se a bandeja pediu alguma tela (e a esquece).
// O Tauri injeta o `State` por valor.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
#[specta::specta]
pub fn take_pending_navigation(pending: State<'_, PendingNav>) -> Option<String> {
    pending.take()
}

/// O que fazer quando o usuário fecha a janela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    /// Fecha e, sendo a única janela, o app encerra.
    Exit,
    /// Fecha a janela e o app continua na bandeja.
    ToTray,
    /// Primeira vez: mostra o aviso, espera um pouco e então fecha para a bandeja.
    ToTrayWithHint,
}

pub const fn close_action(close_to_tray: bool, hint_shown: bool) -> CloseAction {
    match (close_to_tray, hint_shown) {
        (false, _) => CloseAction::Exit,
        (true, true) => CloseAction::ToTray,
        (true, false) => CloseAction::ToTrayWithHint,
    }
}

/// O app abre sem mostrar a janela: pela opção das configurações ou pela flag `--minimized`.
pub fn should_start_minimized<I, S>(args: I, setting: bool) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    setting || args.into_iter().any(|a| a.as_ref() == "--minimized")
}

/// Mostra a janela principal; se ela não existe (começou minimizado ou foi fechada para a
/// bandeja), cria a partir da configuração do `tauri.conf.json`.
pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let Some(config) = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == MAIN_WINDOW)
        .cloned()
    else {
        tracing::error!("a janela principal não está no tauri.conf.json");
        return;
    };
    match WebviewWindowBuilder::from_config(app, &config).and_then(WebviewWindowBuilder::build) {
        Ok(window) => {
            let transparency = app
                .try_state::<AppState>()
                .is_none_or(|s| s.core.settings().get().transparency);
            crate::window::reapply(app, transparency);
            let _ = window.set_focus();
        }
        Err(error) => tracing::error!(%error, "não consegui criar a janela principal"),
    }
}

/// Abre a janela (criando-a, se preciso) na tela `path`.
pub fn navigate<R: Runtime>(app: &AppHandle<R>, path: &str) {
    if let Some(pending) = app.try_state::<PendingNav>() {
        pending.set(path);
    }
    show_main_window(app);
    if let Err(error) = (TrayNavigate {}).emit(app) {
        tracing::warn!(%error, "não consegui avisar a UI da navegação pedida");
    }
}

/// Fechar a janela principal: com "fechar vai para a bandeja", a janela é destruída e o app segue.
pub fn on_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    if window.label() != MAIN_WINDOW {
        return;
    }
    let app = window.app_handle().clone();
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let settings = state.core.settings().get();
    if close_action(settings.close_to_tray, settings.tray_hint_shown) != CloseAction::ToTrayWithHint
    {
        return;
    }
    // O aviso aparece na própria janela; ela fecha logo depois.
    api.prevent_close();
    if let Err(error) = (TrayHint {}).emit(&app) {
        tracing::warn!(%error, "não consegui mostrar o aviso da bandeja");
    }
    let core = std::sync::Arc::clone(&state.core);
    let window = window.clone();
    tauri::async_runtime::spawn(async move {
        let _ = core.settings().modify(|s| s.tray_hint_shown = true).await;
        tokio::time::sleep(HINT_VISIBLE).await;
        let _ = window.destroy();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fechar_sem_bandeja_encerra_o_app() {
        assert_eq!(close_action(false, false), CloseAction::Exit);
        assert_eq!(close_action(false, true), CloseAction::Exit);
    }

    #[test]
    fn a_primeira_vez_na_bandeja_mostra_o_aviso_e_as_outras_nao() {
        assert_eq!(close_action(true, false), CloseAction::ToTrayWithHint);
        assert_eq!(close_action(true, true), CloseAction::ToTray);
    }

    #[test]
    fn iniciar_minimizado_vem_da_opcao_ou_da_flag() {
        assert!(!should_start_minimized(["padgaming.exe"], false));
        assert!(should_start_minimized(["padgaming.exe"], true));
        assert!(should_start_minimized(
            ["padgaming.exe", "--minimized"],
            false
        ));
        assert!(!should_start_minimized(
            ["padgaming.exe", "--minimizedx"],
            false
        ));
    }

    #[test]
    fn a_navegacao_pedida_e_lida_uma_vez_so() {
        let pending = PendingNav::default();
        assert_eq!(pending.take(), None);
        pending.set("/devices?pair=1");
        pending.set("/settings");
        assert_eq!(pending.take().as_deref(), Some("/settings"));
        assert_eq!(pending.take(), None);
    }
}
