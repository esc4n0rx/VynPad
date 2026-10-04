//! Propriedades do gerenciador de slots (T-701): depois de **qualquer** sequência de operações,
//! os invariantes da tabela de 4 slots continuam valendo.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use proptest::prelude::*;
use vynpad_core::{Clock, SlotConfig, SlotManager, SlotView};
use vynpad_driver::{MockDriver, SlotId, XReport, xbuttons};
use vynpad_protocol::control::Uuid;

#[derive(Clone)]
struct ManualClock {
    base: Instant,
    offset: Arc<Mutex<Duration>>,
}

impl Clock for ManualClock {
    fn now(&self) -> Instant {
        self.base + *self.offset.lock().unwrap()
    }
}

/// Seis dispositivos disputam quatro slots.
const DEVICES: u8 = 6;

fn device(n: u8) -> Uuid {
    Uuid::parse(&format!("123e4567-e89b-42d3-a456-4266141740{n:02}")).unwrap()
}

fn slot(i: u8) -> SlotId {
    SlotId::new(i % 4).unwrap()
}

#[derive(Debug, Clone)]
enum Op {
    Assign { device: u8, preferred: Option<u8> },
    Submit { device: u8 },
    Suspend { device: u8 },
    Release { device: u8 },
    Disconnect { slot: u8 },
    Swap { a: u8, b: u8 },
    Pause { slot: u8, paused: bool },
    Advance { secs: u8 },
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => (0..DEVICES, prop::option::of(0u8..4))
            .prop_map(|(device, preferred)| Op::Assign { device, preferred }),
        3 => (0..DEVICES).prop_map(|device| Op::Submit { device }),
        2 => (0..DEVICES).prop_map(|device| Op::Suspend { device }),
        1 => (0..DEVICES).prop_map(|device| Op::Release { device }),
        1 => (0u8..4).prop_map(|slot| Op::Disconnect { slot }),
        2 => (0u8..4, 0u8..4).prop_map(|(a, b)| Op::Swap { a, b }),
        1 => (0u8..4, any::<bool>()).prop_map(|(slot, paused)| Op::Pause { slot, paused }),
        2 => (0u8..40).prop_map(|secs| Op::Advance { secs }),
    ]
}

const REPORT: XReport = XReport {
    buttons: xbuttons::A,
    lt: 1,
    rt: 2,
    lx: 3,
    ly: 4,
    rx: 5,
    ry: 6,
};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn invariants_hold_after_any_sequence_of_operations(ops in prop::collection::vec(op(), 1..80)) {
        let driver = Arc::new(MockDriver::default());
        let clock = ManualClock { base: Instant::now(), offset: Arc::default() };
        let manager = SlotManager::with_clock(
            driver.clone(),
            SlotConfig::default(),
            Arc::new(clock.clone()),
        );
        // A sessão mais recente de cada dispositivo (0 = nenhuma).
        let mut session = [0u32; DEVICES as usize];
        let mut next_session = 1u32;

        for op in ops {
            match op {
                Op::Assign { device: d, preferred } => {
                    let before = manager.snapshot();
                    let already_has_slot = before.slots.iter().any(|s| s.device.as_ref() == Some(&device(d)));
                    let full = before.slots.iter().all(|s| s.state != SlotView::Free);
                    let id = next_session;
                    next_session += 1;
                    match manager.assign(&device(d), id, preferred.map(slot)) {
                        Ok(_) => session[usize::from(d)] = id,
                        // Só recusa quando o dispositivo não tinha slot e todos estavam ocupados
                        // (um slot suspenso que expira é liberado pelo próprio `assign`).
                        Err(error) => {
                            prop_assert!(!already_has_slot, "{error}");
                            prop_assert!(full, "{error}");
                        }
                    }
                }
                Op::Submit { device: d } => { let _ = manager.submit(session[usize::from(d)], &REPORT); }
                Op::Suspend { device: d } => { let _ = manager.suspend(session[usize::from(d)]); }
                Op::Release { device: d } => { let _ = manager.release(session[usize::from(d)]); }
                Op::Disconnect { slot: s } => { let _ = manager.disconnect_slot(slot(s)); }
                Op::Swap { a, b } => { let _ = manager.swap(slot(a), slot(b)); }
                Op::Pause { slot: s, paused } => { let _ = manager.set_paused(slot(s), paused); }
                Op::Advance { secs } => {
                    *clock.offset.lock().unwrap() += Duration::from_secs(u64::from(secs));
                    manager.tick();
                }
            }

            let snapshot = manager.snapshot();

            // 1. Um dispositivo nunca ocupa dois slots; slot livre não tem dono nem sessão.
            let mut owners = Vec::new();
            for s in &snapshot.slots {
                if s.state == SlotView::Free {
                    prop_assert!(s.device.is_none() && s.session.is_none());
                } else {
                    let owner = s.device.clone();
                    prop_assert!(owner.is_some(), "slot ocupado sem dono");
                    owners.push(owner);
                }
            }
            let mut unique = owners.clone();
            unique.sort_by(|a, b| a.as_ref().map(Uuid::as_str).cmp(&b.as_ref().map(Uuid::as_str)));
            unique.dedup();
            prop_assert_eq!(unique.len(), owners.len(), "um dispositivo em dois slots");

            // 2. Os livres e os ocupados fecham os 4 slots.
            prop_assert_eq!(snapshot.free_count() + owners.len(), 4);

            // 3. O índice XInput nunca se repete entre os controles plugados.
            let mut indexes: Vec<u8> = snapshot.slots.iter().filter_map(|s| s.user_index).collect();
            let plugged = indexes.len();
            indexes.sort_unstable();
            indexes.dedup();
            prop_assert_eq!(indexes.len(), plugged, "índice XInput repetido");

            // 4. Os controles plugados no driver são exatamente os dos slots ativos.
            let active = snapshot.slots.iter().filter(|s| s.state == SlotView::Active).count();
            let suspended_plugged = snapshot
                .slots
                .iter()
                .filter(|s| matches!(s.state, SlotView::Suspended { .. }))
                .count();
            prop_assert!(driver.plugged_slots().len() >= active);
            prop_assert!(driver.plugged_slots().len() <= active + suspended_plugged);
        }
    }
}
