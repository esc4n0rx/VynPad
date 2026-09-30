//! Driver de emulação na UI: status, aviso com link de download do `ViGEmBus` e ocupação do
//! `XInput`.
//!
//! Commands: `driver_status`, `open_vigem_download`, `xinput_slots`. Evento: `DriverChanged`
//! (emitido por uma thread que confere o driver a cada segundo, então uma queda do serviço
//! chega à UI em até ~1 s).

use std::time::Duration;

use padgaming_core::{XInputOccupant, classify_xinput};
use padgaming_driver::{DriverStatus, VIGEM_DOWNLOAD_URL};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use crate::error::AppError;
#[cfg(not(windows))]
use crate::error::ErrorCode;
use crate::events::DriverChanged;
use crate::state::AppState;

/// Intervalo entre conferências do driver.
const WATCH_INTERVAL: Duration = Duration::from_secs(1);

/// Status do driver como o frontend o recebe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DriverStatusDto {
    Ready,
    /// Sem o driver: a UI avisa e oferece `download_url`.
    NotInstalled {
        download_url: String,
    },
    Outdated {
        found: String,
        required: String,
        download_url: String,
    },
    ServiceStopped,
    Error {
        message: String,
    },
}

impl From<DriverStatus> for DriverStatusDto {
    fn from(status: DriverStatus) -> Self {
        match status {
            DriverStatus::Ready => Self::Ready,
            DriverStatus::NotInstalled => Self::NotInstalled {
                download_url: VIGEM_DOWNLOAD_URL.to_owned(),
            },
            DriverStatus::Outdated { found, required } => Self::Outdated {
                found,
                required,
                download_url: VIGEM_DOWNLOAD_URL.to_owned(),
            },
            DriverStatus::ServiceStopped => Self::ServiceStopped,
            DriverStatus::Error(message) => Self::Error { message },
        }
    }
}

// O Tauri entrega `State` por valor; não dá para pegar por referência.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
#[specta::specta]
pub fn driver_status(state: State<'_, AppState>) -> DriverStatusDto {
    state.core.driver_status().into()
}

/// Abre a página oficial de download do `ViGEmBus` no navegador padrão.
///
/// A UI chama quando o usuário clica no aviso de driver ausente; o app não abre o navegador
/// por conta própria.
#[tauri::command]
#[specta::specta]
pub fn open_vigem_download() -> Result<(), AppError> {
    #[cfg(windows)]
    {
        tracing::info!(
            url = VIGEM_DOWNLOAD_URL,
            "abrindo a página de download do ViGEmBus"
        );
        Ok(padgaming_driver::setup::open_download_page()?)
    }
    #[cfg(not(windows))]
    {
        Err(AppError::new(
            ErrorCode::Unsupported,
            Some("o ViGEmBus só existe no Windows".to_owned()),
        ))
    }
}

/// Ocupação de um índice `XInput`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct XInputSlotDto {
    pub index: u8,
    pub occupant: XInputOccupantDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum XInputOccupantDto {
    Free,
    Virtual,
    Physical,
}

impl From<XInputOccupant> for XInputOccupantDto {
    fn from(value: XInputOccupant) -> Self {
        match value {
            XInputOccupant::Free => Self::Free,
            XInputOccupant::Virtual => Self::Virtual,
            XInputOccupant::Physical => Self::Physical,
        }
    }
}

/// Quem ocupa os 4 índices `XInput`. A UI chama a 1 Hz enquanto a tela está aberta.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
#[specta::specta]
pub fn xinput_slots(state: State<'_, AppState>) -> Vec<XInputSlotDto> {
    #[cfg(windows)]
    let connected = state
        .xinput
        .as_ref()
        .map_or([false; 4], padgaming_driver::xinput::XInputProbe::connected);
    #[cfg(not(windows))]
    let connected = [false; 4];
    xinput_dtos(connected, &state.core.slots().snapshot())
}

fn xinput_dtos(
    connected: [bool; 4],
    snapshot: &padgaming_core::SlotsSnapshot,
) -> Vec<XInputSlotDto> {
    classify_xinput(connected, snapshot)
        .into_iter()
        .enumerate()
        .map(|(index, occupant)| XInputSlotDto {
            index: u8::try_from(index).unwrap_or(u8::MAX),
            occupant: occupant.into(),
        })
        .collect()
}

/// Devolve o status novo só quando ele difere do último emitido.
fn changed(
    last: &mut Option<DriverStatusDto>,
    current: DriverStatusDto,
) -> Option<DriverStatusDto> {
    if last.as_ref() == Some(&current) {
        return None;
    }
    *last = Some(current.clone());
    Some(current)
}

/// Thread que confere o driver a cada segundo e emite `driver:changed` nas mudanças.
pub fn spawn_watcher(app: &AppHandle) {
    let app = app.clone();
    std::thread::Builder::new()
        .name("driver-watcher".into())
        .spawn(move || {
            let mut last = None;
            loop {
                let status = app.state::<AppState>().core.driver_status().into();
                if let Some(new) = changed(&mut last, status) {
                    if new == DriverStatusDto::Ready {
                        tracing::info!("driver de emulação pronto");
                    } else {
                        tracing::warn!(status = ?new, "driver de emulação indisponível");
                    }
                    let _ = DriverChanged { status: new }.emit(&app);
                }
                std::thread::sleep(WATCH_INTERVAL);
            }
        })
        .expect("thread do monitor do driver");
}

#[cfg(test)]
mod tests {
    use padgaming_driver::MockDriver;
    use serde_json::json;

    use super::*;

    #[test]
    fn status_serializes_with_a_state_tag() {
        let value = |s| serde_json::to_value(DriverStatusDto::from(s)).unwrap();
        assert_eq!(value(DriverStatus::Ready), json!({ "state": "ready" }));
        assert_eq!(
            value(DriverStatus::NotInstalled),
            json!({ "state": "not_installed", "download_url": "https://vigembusdriver.com/download/" })
        );
        assert_eq!(
            value(DriverStatus::ServiceStopped),
            json!({ "state": "service_stopped" })
        );
        assert_eq!(
            value(DriverStatus::Outdated {
                found: "1.9".into(),
                required: "1.16".into()
            }),
            json!({
                "state": "outdated",
                "found": "1.9",
                "required": "1.16",
                "download_url": "https://vigembusdriver.com/download/"
            })
        );
        assert_eq!(
            value(DriverStatus::Error("x".into())),
            json!({ "state": "error", "message": "x" })
        );
    }

    #[test]
    fn watcher_emits_only_on_changes() {
        let mut last = None;
        assert_eq!(
            changed(&mut last, DriverStatusDto::Ready),
            Some(DriverStatusDto::Ready)
        );
        assert_eq!(changed(&mut last, DriverStatusDto::Ready), None);
        assert_eq!(
            changed(&mut last, DriverStatusDto::ServiceStopped),
            Some(DriverStatusDto::ServiceStopped)
        );
        assert_eq!(changed(&mut last, DriverStatusDto::ServiceStopped), None);
    }

    #[test]
    fn xinput_dtos_lists_all_four_indices() {
        let slots = padgaming_core::SlotManager::new(std::sync::Arc::new(MockDriver::default()));
        let dtos = xinput_dtos([true, false, false, true], &slots.snapshot());
        assert_eq!(dtos.len(), 4);
        assert_eq!(dtos[0].occupant, XInputOccupantDto::Physical);
        assert_eq!(dtos[1].occupant, XInputOccupantDto::Free);
        assert_eq!(dtos[3].index, 3);
    }
}
