//! Atualizações do app (T-704): `tauri-plugin-updater` com assinatura minisign e o manifesto
//! `latest.json` no GitHub Releases (endpoint em `tauri.conf.json`).
//!
//! O app procura atualização ao abrir e a cada 6 h (se `check_updates` estiver ligado) e avisa a
//! UI pelo evento `update:available`. Instalar é sempre um pedido do usuário, e **nunca com
//! controles conectados sem confirmação**: o instalador fecha o app, o que derrubaria a partida.

use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};
use tauri_specta::Event;
use vynpad_core::SlotView;
use vynpad_server::Core;

use crate::error::{AppError, ErrorCode};
use crate::state::AppState;

type Result<T> = std::result::Result<T, AppError>;

/// Espera depois da abertura, para a procura não competir com o boot nem com o pareamento.
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(30);
/// Intervalo entre as procuras automáticas.
const CHECK_EVERY: Duration = Duration::from_hours(6);

/// Uma versão nova, como a UI a mostra.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct UpdateInfoDto {
    /// Versão disponível.
    pub version: String,
    /// Versão instalada.
    pub current_version: String,
    /// Notas da versão, em markdown.
    pub notes: Option<String>,
    /// Data da publicação (RFC 3339), se o manifesto trouxer.
    pub date: Option<String>,
}

/// Há uma versão nova (`update:available`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct UpdateAvailable {
    pub update: UpdateInfoDto,
}

/// O que aconteceu com o pedido de instalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct InstallResult {
    /// A instalação começou (o app fecha e reabre sozinho).
    pub started: bool,
    /// Com `started = false`: quantos controles estão conectados, o que exige confirmação.
    pub active_sessions: u32,
}

/// A decisão de instalar agora ou pedir confirmação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    Proceed,
    NeedsConfirmation { active_sessions: u32 },
}

/// Sem controles conectados instala; com controles, só se o usuário já confirmou.
pub const fn install_policy(active_sessions: u32, confirmed: bool) -> Policy {
    if active_sessions == 0 || confirmed {
        Policy::Proceed
    } else {
        Policy::NeedsConfirmation { active_sessions }
    }
}

/// Controles conectados agora: slots com controle plugado ou reservados para uma sessão. Um
/// slot suspenso (conexão caída) não conta, pois não há partida em andamento.
fn active_sessions(core: &Core) -> u32 {
    let snapshot = core.slots().snapshot();
    let count = snapshot
        .slots
        .iter()
        .filter(|s| matches!(s.state, SlotView::Active | SlotView::Reserved))
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// A atualização já encontrada, guardada até o usuário pedir para instalar. Guarda também o
/// `AppHandle` do app real, o que mantém os commands sem tipo genérico no runtime.
pub struct UpdateState {
    app: AppHandle,
    pending: Mutex<Option<(Update, UpdateInfoDto)>>,
}

impl UpdateState {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            pending: Mutex::new(None),
        }
    }

    fn set(&self, found: Option<(Update, UpdateInfoDto)>) {
        *self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = found;
    }

    fn info(&self) -> Option<UpdateInfoDto> {
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(|(_, info)| info.clone())
    }

    fn update(&self) -> Option<Update> {
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(|(update, _)| update.clone())
    }
}

fn failed(what: &str, error: &dyn std::error::Error) -> AppError {
    tracing::warn!(%error, "{what}");
    AppError::new(ErrorCode::UpdateFailed, Some(error.to_string()))
}

/// Pergunta ao servidor de atualizações e guarda o resultado.
async fn check(app: &AppHandle) -> Result<Option<UpdateInfoDto>> {
    let updater = app
        .updater()
        .map_err(|e| failed("updater indisponível", &e))?;
    let found = updater
        .check()
        .await
        .map_err(|e| failed("não consegui procurar atualização", &e))?;
    let state = app.state::<UpdateState>();
    let Some(update) = found else {
        state.set(None);
        return Ok(None);
    };
    let info = UpdateInfoDto {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        notes: update.body.clone(),
        date: update.date.map(|d| d.to_string()),
    };
    let is_new = state.info().as_ref() != Some(&info);
    state.set(Some((update, info.clone())));
    if is_new {
        tracing::info!(version = %info.version, "atualização disponível");
        let _ = UpdateAvailable {
            update: info.clone(),
        }
        .emit(app);
    }
    Ok(Some(info))
}

/// Procura atualização: ao abrir e a cada 6 h, enquanto `check_updates` estiver ligado.
pub fn spawn_checker(app: AppHandle) {
    if crate::e2e_mode() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK_DELAY).await;
        loop {
            let enabled = app
                .try_state::<AppState>()
                .is_some_and(|state| state.core.settings().get().check_updates);
            if enabled {
                // Sem internet é o caso comum; já foi registrado em `failed`.
                let _ = check(&app).await;
            }
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
}

/// Procura agora (botão em Configurações → Sobre), ligado ou não o automático.
#[tauri::command]
#[specta::specta]
pub async fn check_for_update(updates: State<'_, UpdateState>) -> Result<Option<UpdateInfoDto>> {
    check(&updates.app).await
}

/// A atualização já encontrada, para a UI montar o selo sem esperar o próximo evento.
// O Tauri entrega `State` por valor; não dá para pegar por referência.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
#[specta::specta]
pub fn pending_update(updates: State<'_, UpdateState>) -> Option<UpdateInfoDto> {
    updates.info()
}

/// Baixa e instala a atualização encontrada. Com controles conectados e `confirmed = false` não
/// faz nada e devolve quantos são, para a UI pedir confirmação.
#[tauri::command]
#[specta::specta]
pub async fn install_update(
    state: State<'_, AppState>,
    updates: State<'_, UpdateState>,
    confirmed: bool,
) -> Result<InstallResult> {
    let Some(update) = updates.update() else {
        return Err(AppError::not_found("nenhuma atualização encontrada"));
    };
    if let Policy::NeedsConfirmation { active_sessions } =
        install_policy(active_sessions(&state.core), confirmed)
    {
        return Ok(InstallResult {
            started: false,
            active_sessions,
        });
    }

    tracing::info!(version = %update.version, "baixando a atualização");
    let bytes = update
        .download(|_, _| {}, || {})
        .await
        .map_err(|e| failed("não consegui baixar a atualização", &e))?;

    // O download leva tempo: um celular pode ter conectado nesse intervalo.
    if let Policy::NeedsConfirmation { active_sessions } =
        install_policy(active_sessions(&state.core), confirmed)
    {
        return Ok(InstallResult {
            started: false,
            active_sessions,
        });
    }

    // No Windows o instalador encerra o processo sem passar pelo `RunEvent::Exit`: avisa os
    // clients antes, como no reinício do app.
    state.core.stop().await;
    if let Err(error) = update.install(bytes) {
        // O server já parou; reinicia o app para voltar a funcionar com a versão atual.
        tracing::error!(%error, "a instalação falhou; reiniciando o app");
        updates.app.restart();
    }
    // Fora do Windows o `install` só troca os arquivos: reinicia para abrir a versão nova.
    updates.app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sem_controles_instala_sem_perguntar() {
        assert_eq!(install_policy(0, false), Policy::Proceed);
    }

    #[test]
    fn com_controles_pede_confirmacao_e_diz_quantos() {
        assert_eq!(
            install_policy(2, false),
            Policy::NeedsConfirmation { active_sessions: 2 }
        );
    }

    #[test]
    fn confirmado_instala_mesmo_com_controles() {
        assert_eq!(install_policy(4, true), Policy::Proceed);
    }

    #[test]
    fn resultado_serializa_no_formato_da_ui() {
        let json = serde_json::to_value(InstallResult {
            started: false,
            active_sessions: 3,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "started": false, "active_sessions": 3 })
        );
    }
}
