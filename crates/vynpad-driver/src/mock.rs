//! Driver de mentira: guarda os reports e deixa o teste injetar vibração.

use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crossbeam_channel::{Receiver, Sender, unbounded};

use crate::{
    DriverError, DriverStatus, MAX_SLOTS, PadHandle, RumbleEvent, SlotId, VirtualPadDriver, XReport,
};

/// Reports gravados por padrão antes de descartar os mais antigos.
const DEFAULT_CAPACITY: usize = 4096;

struct State {
    status: DriverStatus,
    /// Token da ligação atual de cada slot.
    plugged: [Option<u64>; MAX_SLOTS as usize],
    next_token: u64,
    /// Reports em ordem. A capacidade é reservada de início, então `update` não aloca.
    reports: VecDeque<(SlotId, XReport)>,
    capacity: usize,
}

/// Implementação em memória de [`VirtualPadDriver`], para testes.
pub struct MockDriver {
    state: Mutex<State>,
    rumble_tx: Sender<RumbleEvent>,
    rumble_rx: Receiver<RumbleEvent>,
}

impl Default for MockDriver {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }
}

impl MockDriver {
    /// Driver pronto, guardando até `capacity` reports (os mais antigos saem primeiro).
    pub fn with_capacity(capacity: usize) -> Self {
        let (rumble_tx, rumble_rx) = unbounded();
        Self {
            state: Mutex::new(State {
                status: DriverStatus::Ready,
                plugged: [None; MAX_SLOTS as usize],
                next_token: 1,
                reports: VecDeque::with_capacity(capacity),
                capacity,
            }),
            rumble_tx,
            rumble_rx,
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Simula `NotInstalled`, `Outdated` etc.
    pub fn set_status(&self, status: DriverStatus) {
        self.state().status = status;
    }

    /// Todos os reports recebidos, na ordem.
    pub fn reports(&self) -> Vec<(SlotId, XReport)> {
        self.state().reports.iter().copied().collect()
    }

    /// Último report do slot.
    pub fn last_report(&self, slot: SlotId) -> Option<XReport> {
        self.state()
            .reports
            .iter()
            .rev()
            .find(|(s, _)| *s == slot)
            .map(|(_, r)| *r)
    }

    /// Slots com controle ligado.
    pub fn plugged_slots(&self) -> Vec<SlotId> {
        let state = self.state();
        SlotId::all()
            .filter(|s| state.plugged[usize::from(s.index())].is_some())
            .collect()
    }

    /// Simula um jogo pedindo vibração.
    pub fn inject_rumble(&self, slot: SlotId, large: u8, small: u8) {
        // O receptor vive junto do driver, então o envio não falha.
        let _ = self.rumble_tx.send(RumbleEvent { slot, large, small });
    }
}

impl VirtualPadDriver for MockDriver {
    fn status(&self) -> DriverStatus {
        self.state().status.clone()
    }

    fn plug(&self, slot: SlotId) -> Result<PadHandle, DriverError> {
        let mut state = self.state();
        if state.status != DriverStatus::Ready {
            return Err(DriverError::NotReady(state.status.clone()));
        }
        let entry = usize::from(slot.index());
        if state.plugged[entry].is_some() {
            return Err(DriverError::SlotOccupied(slot));
        }
        let token = state.next_token;
        state.next_token += 1;
        state.plugged[entry] = Some(token);
        Ok(PadHandle::new(slot, token))
    }

    fn unplug(&self, handle: PadHandle) -> Result<(), DriverError> {
        let mut state = self.state();
        let entry = usize::from(handle.slot().index());
        if state.plugged[entry] != Some(handle.token()) {
            return Err(DriverError::InvalidHandle);
        }
        state.plugged[entry] = None;
        Ok(())
    }

    fn update(&self, handle: &PadHandle, report: &XReport) -> Result<(), DriverError> {
        let mut state = self.state();
        if state.plugged[usize::from(handle.slot().index())] != Some(handle.token()) {
            return Err(DriverError::InvalidHandle);
        }
        if state.capacity > 0 {
            if state.reports.len() == state.capacity {
                state.reports.pop_front();
            }
            state.reports.push_back((handle.slot(), *report));
        }
        Ok(())
    }

    fn user_index(&self, handle: &PadHandle) -> Option<u8> {
        let state = self.state();
        (state.plugged[usize::from(handle.slot().index())] == Some(handle.token()))
            .then_some(handle.slot().index())
    }

    fn rumble_events(&self) -> Receiver<RumbleEvent> {
        self.rumble_rx.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(i: u8) -> SlotId {
        SlotId::new(i).unwrap()
    }

    #[test]
    fn plug_update_unplug_lifecycle() {
        let driver = MockDriver::default();
        let pad = driver.plug(slot(1)).unwrap();
        assert_eq!(driver.plugged_slots(), [slot(1)]);
        assert_eq!(driver.user_index(&pad), Some(1));

        let report = XReport {
            buttons: crate::xbuttons::A,
            lt: 200,
            ..XReport::default()
        };
        driver.update(&pad, &report).unwrap();
        assert_eq!(driver.last_report(slot(1)), Some(report));
        assert_eq!(driver.last_report(slot(0)), None);

        driver.unplug(pad).unwrap();
        assert!(driver.plugged_slots().is_empty());
    }

    #[test]
    fn slot_cannot_be_plugged_twice() {
        let driver = MockDriver::default();
        let _pad = driver.plug(slot(0)).unwrap();
        assert_eq!(
            driver.plug(slot(0)).unwrap_err(),
            DriverError::SlotOccupied(slot(0))
        );
    }

    #[test]
    fn stale_handle_is_invalid_after_replug() {
        let driver = MockDriver::default();
        let old = driver.plug(slot(0)).unwrap();
        let old_copy = PadHandle::new(old.slot(), old.token());
        driver.unplug(old).unwrap();
        let _new = driver.plug(slot(0)).unwrap();
        assert_eq!(
            driver.update(&old_copy, &XReport::default()).unwrap_err(),
            DriverError::InvalidHandle
        );
        assert_eq!(driver.user_index(&old_copy), None);
        assert_eq!(
            driver.unplug(old_copy).unwrap_err(),
            DriverError::InvalidHandle
        );
    }

    #[test]
    fn plug_fails_when_driver_is_not_ready() {
        let driver = MockDriver::default();
        driver.set_status(DriverStatus::NotInstalled);
        assert_eq!(driver.status(), DriverStatus::NotInstalled);
        assert_eq!(
            driver.plug(slot(0)).unwrap_err(),
            DriverError::NotReady(DriverStatus::NotInstalled)
        );
    }

    #[test]
    fn rumble_is_delivered_in_order() {
        let driver = MockDriver::default();
        let rx = driver.rumble_events();
        driver.inject_rumble(slot(2), 10, 20);
        driver.inject_rumble(slot(3), 30, 40);
        assert_eq!(
            rx.try_recv().unwrap(),
            RumbleEvent {
                slot: slot(2),
                large: 10,
                small: 20
            }
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            RumbleEvent {
                slot: slot(3),
                large: 30,
                small: 40
            }
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn keeps_only_the_most_recent_reports() {
        let driver = MockDriver::with_capacity(3);
        let pad = driver.plug(slot(0)).unwrap();
        for lx in 0..5 {
            let report = XReport {
                lx,
                ..XReport::default()
            };
            driver.update(&pad, &report).unwrap();
        }
        let lxs: Vec<i16> = driver.reports().iter().map(|(_, r)| r.lx).collect();
        assert_eq!(lxs, [2, 3, 4]);
    }
}
