//! Commands Tauri (frontend → backend). Todos devolvem `Result<_, AppError>`; em `bindings.ts`
//! viram funções que lançam o `AppError`.
//!
//! Regras: nenhum command guarda estado nem decide regra de negócio; eles traduzem tipos e
//! delegam ao `Core` (`padgaming-server`), onde ficam os testes de comportamento.

// O Tauri entrega `State` por valor; não dá para pegar por referência.
#![allow(clippy::needless_pass_by_value)]

use padgaming_driver::SlotId;
use padgaming_net::discovery::interfaces;
use padgaming_storage::AppSettings;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

use crate::dto::{
    AddressDto, DeviceDefaultsDto, DeviceDto, InterfaceDto, LogEntryDto, LogQueryDto, NearbyDto,
    PairingOfferDto, ProfileDto, ProfileInputDto, ProfileSummaryDto, ServerInfoDto, SlotsDto,
};
use crate::error::AppError;
use crate::state::AppState;

/// Limite por mensagem de log vinda da webview (evita inundar o arquivo).
pub(crate) const MAX_FRONTEND_LOG_BYTES: usize = 8 * 1024;

type Result<T> = std::result::Result<T, AppError>;

// ------------------------------------------------------------------ app

#[derive(Serialize, Type)]
pub struct AppInfo {
    pub app_version: &'static str,
    pub core_version: &'static str,
}

#[tauri::command]
#[specta::specta]
pub fn app_info() -> AppInfo {
    AppInfo {
        app_version: env!("CARGO_PKG_VERSION"),
        core_version: padgaming_core::VERSION,
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum FrontendLogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

/// Recebe logs do React e os grava no mesmo arquivo do backend, com target `frontend`.
#[tauri::command]
#[specta::specta]
pub fn log_from_frontend(level: FrontendLogLevel, message: &str) -> Result<()> {
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

// ------------------------------------------------------------------ configurações

#[tauri::command]
#[specta::specta]
pub fn get_settings(state: State<'_, AppState>) -> AppSettings {
    (*state.core.settings().get()).clone()
}

/// Valida, grava e avisa (`SettingsChanged`). Devolve o que ficou gravado.
#[tauri::command]
#[specta::specta]
pub async fn update_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings> {
    let saved = state.core.settings().update(settings).await?;
    Ok((*saved).clone())
}

// ------------------------------------------------------------------ perfis

#[tauri::command]
#[specta::specta]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<Vec<ProfileSummaryDto>> {
    Ok(state
        .core
        .profiles()
        .list()
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn get_profile(state: State<'_, AppState>, id: String) -> Result<ProfileDto> {
    Ok(state.core.profiles().get(&id).await?.into())
}

/// Cria (`id = null`) ou salva um perfil. Salvar aplica na hora nos controles que o usam.
#[tauri::command]
#[specta::specta]
pub async fn save_profile(
    state: State<'_, AppState>,
    id: Option<String>,
    input: ProfileInputDto,
) -> Result<ProfileDto> {
    let profiles = state.core.profiles();
    let stored = match id {
        Some(id) => profiles.save(&id, input.into()).await?,
        None => profiles.create(input.into()).await?,
    };
    Ok(stored.into())
}

/// Apaga um perfil. Em uso, devolve `profile_in_use` (com `in_use`) até `confirm = true`.
#[tauri::command]
#[specta::specta]
pub async fn delete_profile(state: State<'_, AppState>, id: String, confirm: bool) -> Result<()> {
    Ok(state.core.profiles().delete(&id, confirm).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn duplicate_profile(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
) -> Result<ProfileDto> {
    Ok(state
        .core
        .profiles()
        .duplicate(&id, name.as_deref())
        .await?
        .into())
}

/// Importa o conteúdo de um arquivo `.padprofile` como perfil novo.
#[tauri::command]
#[specta::specta]
pub async fn import_profile(state: State<'_, AppState>, text: String) -> Result<ProfileDto> {
    Ok(state.core.profiles().import(&text).await?.into())
}

/// O conteúdo do `.padprofile` para a UI salvar em arquivo.
#[tauri::command]
#[specta::specta]
pub async fn export_profile(state: State<'_, AppState>, id: String) -> Result<String> {
    Ok(state.core.profiles().export(&id).await?)
}

/// "Restaurar padrão": repõe o perfil embutido e aplica nos controles que o usam.
#[tauri::command]
#[specta::specta]
pub async fn restore_default_profile(state: State<'_, AppState>) -> Result<()> {
    Ok(state.core.profiles().restore_builtin().await?)
}

// ------------------------------------------------------------------ dispositivos

#[tauri::command]
#[specta::specta]
pub async fn list_devices(state: State<'_, AppState>) -> Result<Vec<DeviceDto>> {
    let snapshot = state.core.slots().snapshot();
    let current = |id: &str| {
        snapshot
            .slots
            .iter()
            .find(|s| s.device.as_ref().is_some_and(|d| d.as_str() == id))
            .map(|s| s.slot.index())
    };
    Ok(state
        .core
        .devices()
        .list()
        .await?
        .into_iter()
        .map(|device| {
            let slot = current(&device.id);
            DeviceDto::new(device, slot)
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn rename_device(state: State<'_, AppState>, id: String, name: String) -> Result<String> {
    Ok(state.core.rename_device(&id, &name).await?)
}

/// Revoga o dispositivo: o token deixa de valer e a sessão em andamento é encerrada.
#[tauri::command]
#[specta::specta]
pub async fn revoke_device(state: State<'_, AppState>, id: String) -> Result<()> {
    Ok(state.core.revoke_device(&id).await?)
}

/// Esquece o dispositivo (apaga o registro e o histórico): ele precisa parear de novo.
#[tauri::command]
#[specta::specta]
pub async fn forget_device(state: State<'_, AppState>, id: String) -> Result<()> {
    Ok(state.core.forget_device(&id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_device_defaults(
    state: State<'_, AppState>,
    id: String,
    defaults: DeviceDefaultsDto,
) -> Result<()> {
    Ok(state
        .core
        .set_device_defaults(&id, &defaults.into())
        .await?)
}

#[tauri::command]
#[specta::specta]
pub async fn get_connection_log(
    state: State<'_, AppState>,
    query: LogQueryDto,
) -> Result<Vec<LogEntryDto>> {
    Ok(state
        .core
        .log()
        .list(&query.into())
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

// ------------------------------------------------------------------ slots

#[tauri::command]
#[specta::specta]
pub fn get_slots(state: State<'_, AppState>) -> SlotsDto {
    SlotsDto::from(&*state.core.slots().snapshot())
}

fn slot_id(slot: u8) -> Result<SlotId> {
    SlotId::new(slot).ok_or_else(|| AppError::invalid_input("o slot precisa ser de 0 a 3"))
}

/// Desconecta quem está no slot (`bye{kicked}`) e o libera.
#[tauri::command]
#[specta::specta]
pub fn disconnect_slot(state: State<'_, AppState>, slot: u8) -> Result<()> {
    Ok(state.core.disconnect_slot(slot_id(slot)?)?)
}

/// Pausa ou retoma o slot: pausado, o input é ignorado e o controle fica neutro.
#[tauri::command]
#[specta::specta]
pub fn set_slot_paused(state: State<'_, AppState>, slot: u8, paused: bool) -> Result<()> {
    state
        .core
        .slots()
        .set_paused(slot_id(slot)?, paused)
        .map_err(|e| AppError::not_found(&e.to_string()))
}

/// Troca dois slots de lugar, com o controle e o perfil de cada um.
#[tauri::command]
#[specta::specta]
pub fn swap_slots(state: State<'_, AppState>, a: u8, b: u8) -> Result<()> {
    Ok(state.core.swap_slots(slot_id(a)?, slot_id(b)?)?)
}

/// O perfil de cada slot (`id`), na ordem P1–P4; `null` onde não houve dispositivo ainda.
#[tauri::command]
#[specta::specta]
pub fn get_slot_profiles(state: State<'_, AppState>) -> Vec<Option<String>> {
    state.core.slot_profiles().to_vec()
}

/// Troca o perfil do slot na hora (vale só até o dispositivo desconectar). Devolve o `id` aplicado.
#[tauri::command]
#[specta::specta]
pub async fn set_slot_profile(
    state: State<'_, AppState>,
    slot: u8,
    profile_id: String,
) -> Result<String> {
    Ok(state
        .core
        .set_slot_profile(slot_id(slot)?, &profile_id)
        .await?)
}

// ------------------------------------------------------------------ pareamento e rede

/// Gera QR code e PIN. Cada chamada invalida o PIN anterior.
#[tauri::command]
#[specta::specta]
pub fn start_pairing(state: State<'_, AppState>) -> Result<PairingOfferDto> {
    Ok(state.core.start_pairing()?.into())
}

#[tauri::command]
#[specta::specta]
pub fn cancel_pairing(state: State<'_, AppState>) {
    state.core.cancel_pairing();
}

/// Responde ao popup "novo dispositivo". `false` se o pedido já expirou.
#[tauri::command]
#[specta::specta]
pub fn answer_approval(state: State<'_, AppState>, request_id: u64, accept: bool) -> bool {
    state.core.answer_approval(request_id, accept)
}

#[tauri::command]
#[specta::specta]
pub fn get_nearby_devices(state: State<'_, AppState>) -> Vec<NearbyDto> {
    state
        .core
        .discovery()
        .map(|d| d.nearby().into_iter().map(Into::into).collect())
        .unwrap_or_default()
}

#[tauri::command]
#[specta::specta]
pub fn get_server_info(state: State<'_, AppState>) -> ServerInfoDto {
    let core = &state.core;
    let ports = core.ports();
    let info = core
        .discovery()
        .map(padgaming_net::discovery::Discovery::info);
    ServerInfoDto {
        name: info
            .as_ref()
            .map_or_else(padgaming_server::computer_name, |i| i.wire_name()),
        fingerprint_short: padgaming_protocol::discovery::short_fingerprint(&core.fingerprint()),
        control_port: ports.control,
        input_port: ports.input,
        discovery_port: ports.discovery,
        ports_pending_restart: core.ports_pending_restart(),
        addresses: core
            .local_addresses()
            .into_iter()
            .map(|i| AddressDto {
                interface: i.name,
                ip: i.ip.to_string(),
            })
            .collect(),
        free_slots: u8::try_from(core.slots().snapshot().free_count()).unwrap_or(4),
    }
}

/// As interfaces de rede ativas, para a tela de Configurações (quais o anúncio usa).
#[tauri::command]
#[specta::specta]
pub fn list_network_interfaces() -> Result<Vec<InterfaceDto>> {
    Ok(interfaces::available()?
        .into_iter()
        .map(Into::into)
        .collect())
}

// ------------------------------------------------------------------ input ao vivo

/// Uma tela passa a ver o controle do `slot`: o backend começa a emitir `InputLive`.
#[tauri::command]
#[specta::specta]
pub fn subscribe_live_input(state: State<'_, AppState>, slot: u8) -> Result<()> {
    if state.live.subscribe(slot) {
        Ok(())
    } else {
        Err(slot_id(slot)
            .err()
            .unwrap_or_else(|| AppError::not_found("slot")))
    }
}

/// Zera as assinaturas do input ao vivo. A UI chama ao carregar: uma janela recarregada (HMR, F5)
/// perdeu as assinaturas locais, mas o backend ainda as conta.
#[tauri::command]
#[specta::specta]
pub fn reset_live_input(state: State<'_, AppState>) {
    state.live.reset();
}

/// A tela deixou de ver o `slot`. Sem nenhuma tela assinando, o backend para de emitir.
#[tauri::command]
#[specta::specta]
pub fn unsubscribe_live_input(state: State<'_, AppState>, slot: u8) {
    state.live.unsubscribe(slot);
}
