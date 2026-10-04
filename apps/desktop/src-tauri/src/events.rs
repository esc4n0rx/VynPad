//! Eventos tipados backend → frontend (gerados em `bindings.ts` como `events.*`).
//!
//! O `Core` publica um único fluxo de [`CoreEvent`]; [`translate`] o traduz para os eventos
//! abaixo e [`spawn_pump`] os emite para a webview.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;
use tokio::sync::broadcast::error::RecvError;
use vynpad_net::control::ControlEvent;
use vynpad_server::{ApprovalEvent, Core, CoreEvent};
use vynpad_storage::AppSettings;

use crate::driver::DriverStatusDto;
use crate::dto::{
    ApprovalPromptDto, InputReportDto, MetricsDto, NearbyDto, SessionHealthDto, SlotsDto,
};

/// As configurações mudaram (`settings:changed`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct SettingsChanged {
    pub settings: AppSettings,
}

/// Os 4 slots mudaram (`slots:changed`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct SlotsChanged {
    pub state: SlotsDto,
}

/// A lista de dispositivos pareados mudou (novo, renomeado, revogado, esquecido, visto).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct DevicesChanged {}

/// Métricas de latência dos slots (`metrics:tick`): 4 Hz com a janela à vista, 0,2 Hz sem.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct MetricsTick {
    pub metrics: MetricsDto,
}

/// Um dispositivo terminou o primeiro pareamento (`devices:paired`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct DevicesPaired {
    pub device_id: String,
    pub name: String,
    pub model: String,
}

/// A lista de dispositivos próximos mudou (`devices:discovered`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct DevicesDiscovered {
    pub devices: Vec<NearbyDto>,
}

/// Uma sessão começou.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct SessionStarted {
    pub session_id: u32,
    pub slot: u8,
    pub device_id: String,
    pub peer: String,
}

/// Uma sessão terminou.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct SessionEnded {
    pub session_id: u32,
    pub slot: u8,
    pub device_id: String,
    /// `dropped`, `timeout`, `client_bye`, `kicked`, `revoked` ou `shutdown`.
    pub reason: String,
}

/// O fluxo de input da sessão mudou de estado (`session:state`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct SessionState {
    pub session_id: u32,
    pub slot: u8,
    pub health: SessionHealthDto,
}

/// Uma conexão foi recusada.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ConnectionRejected {
    pub peer: String,
    pub device_id: Option<String>,
    /// `bad_request`, `bad_auth`, `locked`, `revoked`, `no_slot` ou `version`.
    pub reason: String,
}

/// A reserva de um slot venceu.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ReservationExpired {
    pub slot: u8,
}

/// Um novo dispositivo espera a confirmação do usuário.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ApprovalRequested {
    pub prompt: ApprovalPromptDto,
}

/// O pedido de confirmação acabou (respondido ou expirou).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ApprovalResolved {
    pub request_id: u64,
    pub accepted: bool,
}

/// O status do driver de controles virtuais mudou (`driver:changed`).
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct DriverChanged {
    pub status: DriverStatusDto,
}

/// O estado de um controle, enquanto alguma tela o assina (`input:live`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct InputLive {
    pub slot: u8,
    pub report: InputReportDto,
}

/// Um evento pronto para emitir.
#[derive(Debug, Clone)]
pub enum Outbound {
    SettingsChanged(SettingsChanged),
    SlotsChanged(SlotsChanged),
    DevicesChanged,
    DevicesPaired(DevicesPaired),
    DevicesDiscovered(DevicesDiscovered),
    SessionStarted(SessionStarted),
    SessionEnded(SessionEnded),
    SessionState(SessionState),
    ConnectionRejected(ConnectionRejected),
    ReservationExpired(ReservationExpired),
    ApprovalRequested(ApprovalRequested),
    ApprovalResolved(ApprovalResolved),
    MetricsTick(MetricsTick),
}

impl Outbound {
    /// Emite para todas as janelas.
    pub fn emit<R: Runtime>(self, app: &AppHandle<R>) {
        let result = match self {
            Self::SettingsChanged(e) => e.emit(app),
            Self::SlotsChanged(e) => e.emit(app),
            Self::DevicesChanged => DevicesChanged {}.emit(app),
            Self::DevicesPaired(e) => e.emit(app),
            Self::DevicesDiscovered(e) => e.emit(app),
            Self::SessionStarted(e) => e.emit(app),
            Self::SessionEnded(e) => e.emit(app),
            Self::SessionState(e) => e.emit(app),
            Self::ConnectionRejected(e) => e.emit(app),
            Self::ReservationExpired(e) => e.emit(app),
            Self::ApprovalRequested(e) => e.emit(app),
            Self::ApprovalResolved(e) => e.emit(app),
            Self::MetricsTick(e) => e.emit(app),
        };
        if let Err(error) = result {
            tracing::warn!(%error, "não consegui emitir um evento para a UI");
        }
    }
}

/// O texto estável do motivo de fim de sessão.
const fn end_reason(reason: vynpad_net::control::SessionEnd) -> &'static str {
    use vynpad_net::control::SessionEnd::{ClientBye, Dropped, Kicked, Revoked, Shutdown, Timeout};
    match reason {
        Dropped => "dropped",
        Timeout => "timeout",
        ClientBye => "client_bye",
        Kicked => "kicked",
        Revoked => "revoked",
        Shutdown => "shutdown",
    }
}

fn reject_reason(reason: vynpad_protocol::control::RejectReason) -> &'static str {
    use vynpad_protocol::control::RejectReason::{
        BadAuth, BadRequest, Locked, NoSlot, Revoked, Unknown, Version,
    };
    match reason {
        BadRequest => "bad_request",
        BadAuth => "bad_auth",
        Locked => "locked",
        Revoked => "revoked",
        NoSlot => "no_slot",
        Version => "version",
        Unknown => "unknown",
    }
}

/// Traduz um evento do `Core` para os que a UI recebe (às vezes nenhum, às vezes mais de um).
pub fn translate(event: &CoreEvent) -> Vec<Outbound> {
    match event {
        CoreEvent::Slots(snapshot) => vec![Outbound::SlotsChanged(SlotsChanged {
            state: SlotsDto::from(&**snapshot),
        })],
        CoreEvent::Settings(settings) => vec![Outbound::SettingsChanged(SettingsChanged {
            settings: AppSettings::clone(settings),
        })],
        CoreEvent::Nearby(list) => vec![Outbound::DevicesDiscovered(DevicesDiscovered {
            devices: list.iter().cloned().map(NearbyDto::from).collect(),
        })],
        CoreEvent::DevicesChanged => vec![Outbound::DevicesChanged],
        CoreEvent::Approval(ApprovalEvent::Requested(prompt)) => {
            vec![Outbound::ApprovalRequested(ApprovalRequested {
                prompt: prompt.clone().into(),
            })]
        }
        CoreEvent::Approval(ApprovalEvent::Resolved {
            request_id,
            accepted,
        }) => vec![Outbound::ApprovalResolved(ApprovalResolved {
            request_id: *request_id,
            accepted: *accepted,
        })],
        CoreEvent::Control(control) => translate_control(control),
        CoreEvent::Metrics(snapshot) => vec![Outbound::MetricsTick(MetricsTick {
            metrics: MetricsDto::from(&**snapshot),
        })],
    }
}

fn translate_control(event: &ControlEvent) -> Vec<Outbound> {
    match event {
        ControlEvent::Paired {
            device_id,
            name,
            model,
        } => vec![Outbound::DevicesPaired(DevicesPaired {
            device_id: device_id.as_str().to_owned(),
            name: name.clone(),
            model: model.clone(),
        })],
        ControlEvent::SessionStarted {
            session_id,
            slot,
            device_id,
            peer,
        } => vec![Outbound::SessionStarted(SessionStarted {
            session_id: *session_id,
            slot: slot.index(),
            device_id: device_id.as_str().to_owned(),
            peer: peer.to_string(),
        })],
        ControlEvent::SessionEnded {
            session_id,
            slot,
            device_id,
            reason,
        } => vec![Outbound::SessionEnded(SessionEnded {
            session_id: *session_id,
            slot: slot.index(),
            device_id: device_id.as_str().to_owned(),
            reason: end_reason(*reason).to_owned(),
        })],
        ControlEvent::SessionState {
            session_id,
            slot,
            health,
        } => vec![Outbound::SessionState(SessionState {
            session_id: *session_id,
            slot: slot.index(),
            health: (*health).into(),
        })],
        ControlEvent::Rejected {
            peer,
            device_id,
            reason,
        } => vec![Outbound::ConnectionRejected(ConnectionRejected {
            peer: peer.to_string(),
            device_id: device_id.as_ref().map(|d| d.as_str().to_owned()),
            reason: reject_reason(*reason).to_owned(),
        })],
        ControlEvent::ReservationExpired { slot } => {
            vec![Outbound::ReservationExpired(ReservationExpired {
                slot: slot.index(),
            })]
        }
        // O status do driver tem o próprio monitor (`driver:changed`).
        ControlEvent::DriverRechecked(_) => Vec::new(),
    }
}

/// Encaminha os eventos do `Core` para a webview.
pub fn spawn_pump<R: Runtime>(app: AppHandle<R>, core: &Arc<Core>) {
    let mut events = core.subscribe();
    let mut transparency = core.settings().get().transparency;
    let mut autostart = core.settings().get().autostart;
    tauri::async_runtime::spawn(async move {
        loop {
            match events.recv().await {
                Ok(event) => {
                    // O efeito de vidro da janela segue a preferência de transparência.
                    if let CoreEvent::Settings(settings) = &event
                        && settings.transparency != transparency
                    {
                        transparency = settings.transparency;
                        crate::window::reapply(&app, transparency);
                    }
                    // Iniciar com o sistema segue o interruptor, sem reiniciar o app.
                    if let CoreEvent::Settings(settings) = &event
                        && settings.autostart != autostart
                    {
                        autostart = settings.autostart;
                        crate::autostart::apply(&app, autostart);
                    }
                    for outbound in translate(&event) {
                        outbound.emit(&app);
                    }
                }
                Err(RecvError::Lagged(missed)) => {
                    // A UI perdeu eventos: ela precisa reler o estado. Um `DevicesChanged` e o
                    // próximo `SlotsChanged` bastam para refazer as consultas.
                    tracing::warn!(missed, "a ponte com a UI ficou para trás");
                    Outbound::DevicesChanged.emit(&app);
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use vynpad_core::SlotManager;
    use vynpad_driver::{MockDriver, SlotId};
    use vynpad_protocol::control::{RejectReason, Uuid};

    use super::*;

    fn slot(i: u8) -> SlotId {
        SlotId::new(i).unwrap()
    }

    fn id() -> Uuid {
        Uuid::parse("00000000-0000-4000-8000-000000000001").unwrap()
    }

    #[test]
    fn control_events_become_the_typed_ui_events() {
        let peer: IpAddr = "192.168.0.9".parse().unwrap();
        let started = translate(&CoreEvent::Control(ControlEvent::SessionStarted {
            session_id: 5,
            slot: slot(2),
            device_id: id(),
            peer,
        }));
        assert!(matches!(
            &started[..],
            [Outbound::SessionStarted(e)]
                if e.session_id == 5 && e.slot == 2 && e.peer == "192.168.0.9"
        ));

        let rejected = translate(&CoreEvent::Control(ControlEvent::Rejected {
            peer,
            device_id: None,
            reason: RejectReason::BadAuth,
        }));
        assert!(matches!(
            &rejected[..],
            [Outbound::ConnectionRejected(e)] if e.reason == "bad_auth" && e.device_id.is_none()
        ));

        let ended = translate(&CoreEvent::Control(ControlEvent::SessionEnded {
            session_id: 5,
            slot: slot(2),
            device_id: id(),
            reason: vynpad_net::control::SessionEnd::Revoked,
        }));
        assert!(matches!(&ended[..], [Outbound::SessionEnded(e)] if e.reason == "revoked"));

        let state = translate(&CoreEvent::Control(ControlEvent::SessionState {
            session_id: 5,
            slot: slot(2),
            health: vynpad_net::control::SessionHealth::Stalled,
        }));
        assert!(matches!(
            &state[..],
            [Outbound::SessionState(e)] if e.health == SessionHealthDto::Stalled
        ));
    }

    #[test]
    fn slot_and_settings_events_carry_the_full_state() {
        let manager = SlotManager::new(Arc::new(MockDriver::default()));
        manager.assign(&id(), 1, None).unwrap();
        let out = translate(&CoreEvent::Slots(manager.snapshot()));
        let [Outbound::SlotsChanged(e)] = &out[..] else {
            panic!("{out:?}");
        };
        assert_eq!(e.state.slots.len(), 4);
        assert_eq!(e.state.free_count, 3);
        assert_eq!(e.state.slots[0].device_id.as_deref(), Some(id().as_str()));

        let settings = Arc::new(AppSettings::default());
        assert!(matches!(
            &translate(&CoreEvent::Settings(settings))[..],
            [Outbound::SettingsChanged(_)]
        ));
    }

    #[test]
    fn the_drivers_own_monitor_is_not_duplicated() {
        let out = translate(&CoreEvent::Control(ControlEvent::DriverRechecked(
            vynpad_driver::DriverStatus::Ready,
        )));
        assert!(out.is_empty());
    }
}
