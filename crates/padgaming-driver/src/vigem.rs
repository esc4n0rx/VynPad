//! Backend `ViGEmBus` (Windows): controles Xbox 360 virtuais via `vigem-client`.
//!
//! Um `Client` (handle do barramento) é compartilhado e cada slot tem seu `Xbox360Wired`.
//! Vibração: cada controle ligado tem uma thread que espera as notificações do driver e as
//! repassa ao canal de [`VirtualPadDriver::rumble_events`].
//!
//! Remoção: `Drop` desliga o controle e encerra a thread de vibração. Se o processo morrer sem
//! rodar destrutores, o `ViGEmBus` remove os controles quando o handle do client é fechado.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, unbounded};
use vigem_client::{Client, Error as VigemError, TargetId, XButtons, XGamepad, Xbox360Wired};

use crate::{
    DriverError, DriverStatus, MAX_SLOTS, PadHandle, RumbleEvent, SlotId, VirtualPadDriver, XReport,
};

/// Quanto esperar um controle novo aceitar reports.
const SETTLE_TIMEOUT: Duration = Duration::from_secs(2);

type Target = Xbox360Wired<Arc<Client>>;

/// Versão mínima do `ViGEmBus` mostrada quando o client da crate recusa o driver instalado.
const REQUIRED_BUS: &str = "ViGEmBus 1.16 ou superior";

/// Controle ligado em um slot.
struct Pad {
    token: u64,
    target: Option<Target>,
    notifier: Option<JoinHandle<()>>,
}

impl Drop for Pad {
    fn drop(&mut self) {
        // Primeiro o controle: é o que aborta a espera da thread de vibração.
        drop(self.target.take());
        if let Some(handle) = self.notifier.take() {
            let _ = handle.join();
        }
    }
}

/// [`VirtualPadDriver`] sobre o `ViGEmBus`.
pub struct VigemDriver {
    client: Mutex<Option<Arc<Client>>>,
    status: Mutex<DriverStatus>,
    pads: [Mutex<Option<Pad>>; MAX_SLOTS as usize],
    next_token: AtomicU64,
    rumble_tx: Sender<RumbleEvent>,
    rumble_rx: Receiver<RumbleEvent>,
}

impl Default for VigemDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl VigemDriver {
    /// Tenta conectar ao barramento. Nunca falha: sem o driver, `status()` diz o motivo e a
    /// conexão é tentada de novo a cada `status()`/`plug()`.
    pub fn new() -> Self {
        let (rumble_tx, rumble_rx) = unbounded();
        let driver = Self {
            client: Mutex::new(None),
            status: Mutex::new(DriverStatus::NotInstalled),
            pads: Default::default(),
            next_token: AtomicU64::new(1),
            rumble_tx,
            rumble_rx,
        };
        let _ = driver.connect_client();
        driver
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Conexão com o barramento, criando-a se ainda não existir.
    fn connect_client(&self) -> Result<Arc<Client>, DriverStatus> {
        let mut client = Self::lock(&self.client);
        if let Some(existing) = client.as_ref() {
            return Ok(Arc::clone(existing));
        }
        match Client::connect() {
            Ok(connected) => {
                let connected = Arc::new(connected);
                *client = Some(Arc::clone(&connected));
                *Self::lock(&self.status) = DriverStatus::Ready;
                Ok(connected)
            }
            Err(error) => {
                let status = status_from_error(error);
                *Self::lock(&self.status) = status.clone();
                Err(status)
            }
        }
    }

    /// O driver falhou no meio do uso (serviço parado, por exemplo): fica registrado em `status()`.
    fn note_failure(&self, error: VigemError) {
        tracing::error!(%error, "ViGEmBus falhou durante o uso");
        *Self::lock(&self.status) = DriverStatus::Error(error.to_string());
    }
}

fn status_from_error(error: VigemError) -> DriverStatus {
    match error {
        VigemError::BusNotFound => DriverStatus::NotInstalled,
        VigemError::BusVersionMismatch => DriverStatus::Outdated {
            found: "incompatível".to_owned(),
            required: REQUIRED_BUS.to_owned(),
        },
        other => DriverStatus::Error(other.to_string()),
    }
}

fn to_gamepad(report: &XReport) -> XGamepad {
    XGamepad {
        buttons: XButtons {
            raw: report.buttons,
        },
        left_trigger: report.lt,
        right_trigger: report.rt,
        thumb_lx: report.lx,
        thumb_ly: report.ly,
        thumb_rx: report.rx,
        thumb_ry: report.ry,
    }
}

/// Erros transitórios de um controle recém-criado: o driver ainda não o expõe ao `XInput`.
fn is_not_ready(error: VigemError) -> bool {
    // 483 = ERROR_DEVICE_NOT_CONNECTED
    matches!(
        error,
        VigemError::TargetNotReady | VigemError::WinError(483)
    )
}

/// Mesmo depois de `wait_ready` o primeiro report pode falhar por alguns milissegundos.
/// Espera o controle aceitar um report neutro, para o `update` do hot path já nascer estável.
fn settle(target: &mut Target) -> Result<(), DriverError> {
    let deadline = Instant::now() + SETTLE_TIMEOUT;
    loop {
        match target.update(&XGamepad::default()) {
            Ok(()) => return Ok(()),
            Err(error) if is_not_ready(error) && Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(backend(error)),
        }
    }
}

fn backend(error: VigemError) -> DriverError {
    DriverError::Backend(error.to_string())
}

impl VirtualPadDriver for VigemDriver {
    fn status(&self) -> DriverStatus {
        if Self::lock(&self.client).is_none() {
            let _ = self.connect_client();
        }
        Self::lock(&self.status).clone()
    }

    fn plug(&self, slot: SlotId) -> Result<PadHandle, DriverError> {
        let client = self.connect_client().map_err(DriverError::NotReady)?;
        let mut entry = Self::lock(&self.pads[usize::from(slot.index())]);
        if entry.is_some() {
            return Err(DriverError::SlotOccupied(slot));
        }

        let mut target = Xbox360Wired::new(client, TargetId::XBOX360_WIRED);
        target.plugin().map_err(backend)?;
        // Se algo falhar daqui em diante, o `Drop` do target desliga o controle.
        target.wait_ready().map_err(backend)?;
        settle(&mut target)?;

        let tx = self.rumble_tx.clone();
        let notifier = target
            .request_notification()
            .map_err(backend)?
            .spawn_thread(move |_, n| {
                let _ = tx.send(RumbleEvent {
                    slot,
                    large: n.large_motor,
                    small: n.small_motor,
                });
            });

        let token = self.next_token.fetch_add(1, Ordering::Relaxed);
        *entry = Some(Pad {
            token,
            target: Some(target),
            notifier: Some(notifier),
        });
        *Self::lock(&self.status) = DriverStatus::Ready;
        Ok(PadHandle::new(slot, token))
    }

    fn unplug(&self, handle: PadHandle) -> Result<(), DriverError> {
        let pad = {
            let mut entry = Self::lock(&self.pads[usize::from(handle.slot().index())]);
            if entry.as_ref().map(|p| p.token) != Some(handle.token()) {
                return Err(DriverError::InvalidHandle);
            }
            entry.take()
        };
        // O `Drop` espera a thread de vibração; fora do lock para não travar o slot.
        drop(pad);
        Ok(())
    }

    fn update(&self, handle: &PadHandle, report: &XReport) -> Result<(), DriverError> {
        let mut entry = Self::lock(&self.pads[usize::from(handle.slot().index())]);
        let target = entry
            .as_mut()
            .filter(|p| p.token == handle.token())
            .and_then(|p| p.target.as_mut())
            .ok_or(DriverError::InvalidHandle)?;
        target.update(&to_gamepad(report)).map_err(|error| {
            match error {
                VigemError::NotPluggedIn => return DriverError::InvalidHandle,
                _ if is_not_ready(error) => {}
                _ => self.note_failure(error),
            }
            backend(error)
        })
    }

    fn user_index(&self, handle: &PadHandle) -> Option<u8> {
        let mut entry = Self::lock(&self.pads[usize::from(handle.slot().index())]);
        let target = entry
            .as_mut()
            .filter(|p| p.token == handle.token())?
            .target
            .as_mut()?;
        let index = u8::try_from(target.get_user_index().ok()?).ok()?;
        (index < MAX_SLOTS).then_some(index)
    }

    fn rumble_events(&self) -> Receiver<RumbleEvent> {
        self.rumble_rx.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_report_field() {
        let report = XReport {
            buttons: 0x1234,
            lt: 1,
            rt: 2,
            lx: -3,
            ly: 4,
            rx: -5,
            ry: 6,
        };
        let pad = to_gamepad(&report);
        assert_eq!(pad.buttons.raw, 0x1234);
        assert_eq!((pad.left_trigger, pad.right_trigger), (1, 2));
        assert_eq!((pad.thumb_lx, pad.thumb_ly), (-3, 4));
        assert_eq!((pad.thumb_rx, pad.thumb_ry), (-5, 6));
    }

    #[test]
    fn bus_errors_become_driver_status() {
        assert_eq!(
            status_from_error(VigemError::BusNotFound),
            DriverStatus::NotInstalled
        );
        assert!(matches!(
            status_from_error(VigemError::BusVersionMismatch),
            DriverStatus::Outdated { .. }
        ));
        assert!(matches!(
            status_from_error(VigemError::BusAccessFailed(5)),
            DriverStatus::Error(_)
        ));
    }
}
