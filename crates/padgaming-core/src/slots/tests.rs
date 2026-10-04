use std::sync::Mutex;

use padgaming_driver::{DriverStatus, MockDriver, xbuttons};

use super::*;

#[derive(Clone)]
struct TestClock {
    base: Instant,
    offset: Arc<Mutex<Duration>>,
}

impl TestClock {
    fn new() -> Self {
        Self {
            base: Instant::now(),
            offset: Arc::default(),
        }
    }

    fn advance(&self, by: Duration) {
        *self.offset.lock().unwrap() += by;
    }
}

impl Clock for TestClock {
    fn now(&self) -> Instant {
        self.base + *self.offset.lock().unwrap()
    }
}

struct Fixture {
    manager: SlotManager,
    driver: Arc<MockDriver>,
    clock: TestClock,
}

fn fixture() -> Fixture {
    let driver = Arc::new(MockDriver::default());
    let clock = TestClock::new();
    let manager = SlotManager::with_clock(
        driver.clone(),
        SlotConfig::default(),
        Arc::new(clock.clone()),
    );
    Fixture {
        manager,
        driver,
        clock,
    }
}

fn slot(i: u8) -> SlotId {
    SlotId::new(i).unwrap()
}

fn device(n: u8) -> Uuid {
    Uuid::parse(&format!("123e4567-e89b-42d3-a456-4266141740{n:02}")).unwrap()
}

const PRESSED: XReport = XReport {
    buttons: xbuttons::A,
    lt: 10,
    rt: 20,
    lx: 30,
    ly: 40,
    rx: 50,
    ry: 60,
};

fn state(f: &Fixture, i: u8) -> SlotView {
    f.manager.snapshot().slots[usize::from(i)].state
}

// ------------------------------------------------------------ atribuição

#[test]
fn assigns_the_lowest_free_slot() {
    let f = fixture();
    assert_eq!(f.manager.assign(&device(1), 1, None), Ok(slot(0)));
    assert_eq!(f.manager.assign(&device(2), 2, None), Ok(slot(1)));
    assert_eq!(f.manager.assign(&device(3), 3, None), Ok(slot(2)));
}

#[test]
fn preferred_slot_wins_when_free_otherwise_lowest() {
    let f = fixture();
    assert_eq!(f.manager.assign(&device(1), 1, Some(slot(2))), Ok(slot(2)));
    // O preferido já está ocupado: cai para o menor livre.
    assert_eq!(f.manager.assign(&device(2), 2, Some(slot(2))), Ok(slot(0)));
}

#[test]
fn fifth_client_gets_no_slot() {
    let f = fixture();
    for n in 1..=4 {
        f.manager.assign(&device(n), u32::from(n), None).unwrap();
    }
    assert_eq!(
        f.manager.assign(&device(5), 5, None),
        Err(SlotError::NoSlot)
    );
    assert_eq!(f.manager.snapshot().free_count(), 0);
}

#[test]
fn assignment_does_not_plug_a_pad() {
    let f = fixture();
    f.manager.assign(&device(1), 1, None).unwrap();
    assert!(f.driver.plugged_slots().is_empty());
    assert_eq!(state(&f, 0), SlotView::Reserved);
}

// ------------------------------------------------------------ input

#[test]
fn first_input_plugs_the_pad_and_later_inputs_reuse_it() {
    let f = fixture();
    f.manager.assign(&device(1), 7, None).unwrap();
    f.manager.submit(7, &PRESSED).unwrap();
    assert_eq!(f.driver.plugged_slots(), [slot(0)]);
    assert_eq!(state(&f, 0), SlotView::Active);
    assert_eq!(f.driver.last_report(slot(0)), Some(PRESSED));

    let next = XReport { lx: 999, ..PRESSED };
    f.manager.submit(7, &next).unwrap();
    assert_eq!(f.driver.plugged_slots(), [slot(0)]);
    assert_eq!(f.driver.last_report(slot(0)), Some(next));
}

#[test]
fn input_from_an_unknown_session_is_rejected() {
    let f = fixture();
    assert_eq!(
        f.manager.submit(99, &PRESSED),
        Err(SlotError::UnknownSession)
    );
    assert!(f.driver.plugged_slots().is_empty());
}

#[test]
fn driver_failure_on_first_input_leaves_the_slot_reserved_and_retryable() {
    let f = fixture();
    f.driver.set_status(DriverStatus::NotInstalled);
    f.manager.assign(&device(1), 7, None).unwrap();
    assert!(matches!(
        f.manager.submit(7, &PRESSED),
        Err(SlotError::Driver(DriverError::NotReady(_)))
    ));
    assert_eq!(state(&f, 0), SlotView::Reserved);

    f.driver.set_status(DriverStatus::Ready);
    f.manager.submit(7, &PRESSED).unwrap();
    assert_eq!(state(&f, 0), SlotView::Active);
}

// ------------------------------------------------------------ queda e reconexão

#[test]
fn suspend_neutralizes_but_keeps_the_pad_plugged() {
    let f = fixture();
    f.manager.assign(&device(1), 7, None).unwrap();
    f.manager.submit(7, &PRESSED).unwrap();

    assert_eq!(f.manager.suspend(7), Some(slot(0)));
    assert_eq!(f.driver.last_report(slot(0)), Some(XReport::default()));
    assert_eq!(f.driver.plugged_slots(), [slot(0)]);
    assert!(matches!(state(&f, 0), SlotView::Suspended { .. }));
    // A sessão antiga não controla mais nada.
    assert_eq!(
        f.manager.submit(7, &PRESSED),
        Err(SlotError::UnknownSession)
    );
}

#[test]
fn neutralize_session_releases_the_buttons_and_keeps_the_pad_and_the_session() {
    let f = fixture();
    f.manager.assign(&device(1), 7, None).unwrap();
    // Sem controle plugado ainda (nenhum input) e sessão desconhecida: nada a fazer.
    assert!(!f.manager.neutralize_session(7));
    f.manager.submit(7, &PRESSED).unwrap();
    assert!(!f.manager.neutralize_session(99));

    assert!(f.manager.neutralize_session(7));
    assert_eq!(f.driver.last_report(slot(0)), Some(XReport::default()));
    assert_eq!(f.driver.plugged_slots(), [slot(0)]);
    assert_eq!(state(&f, 0), SlotView::Active);
    // A sessão segue válida: o próximo input volta a mexer no controle.
    f.manager.submit(7, &PRESSED).unwrap();
    assert_eq!(f.driver.last_report(slot(0)), Some(PRESSED));
}

#[test]
fn reconnect_within_the_window_returns_to_the_same_slot_without_replug() {
    let f = fixture();
    f.manager.assign(&device(1), 7, None).unwrap();
    f.manager.assign(&device(2), 8, None).unwrap();
    f.manager.submit(8, &PRESSED).unwrap();
    f.manager.suspend(8);
    let reports_before = f.driver.reports().len();

    f.clock.advance(Duration::from_secs(29));
    assert_eq!(f.manager.assign(&device(2), 9, None), Ok(slot(1)));
    assert_eq!(state(&f, 1), SlotView::Active);
    assert_eq!(f.driver.plugged_slots(), [slot(1)], "sem unplug/replug");

    // A nova sessão comanda o mesmo controle; a antiga não.
    f.manager.submit(9, &PRESSED).unwrap();
    assert_eq!(f.driver.last_report(slot(1)), Some(PRESSED));
    assert!(f.driver.reports().len() > reports_before);
    assert_eq!(
        f.manager.submit(8, &PRESSED),
        Err(SlotError::UnknownSession)
    );
}

#[test]
fn reconnect_before_first_input_stays_reserved() {
    let f = fixture();
    f.manager.assign(&device(1), 7, None).unwrap();
    f.manager.suspend(7);
    assert_eq!(f.manager.assign(&device(1), 8, None), Ok(slot(0)));
    assert_eq!(state(&f, 0), SlotView::Reserved);
    assert!(f.driver.plugged_slots().is_empty());
}

#[test]
fn reservation_expires_after_30_seconds_and_unplugs() {
    let f = fixture();
    f.manager.assign(&device(1), 7, None).unwrap();
    f.manager.submit(7, &PRESSED).unwrap();
    f.manager.suspend(7);

    f.clock.advance(Duration::from_secs(29));
    assert!(f.manager.tick().is_empty());
    assert_eq!(f.driver.plugged_slots(), [slot(0)]);

    f.clock.advance(Duration::from_secs(1));
    assert_eq!(f.manager.tick(), [slot(0)]);
    assert_eq!(state(&f, 0), SlotView::Free);
    assert!(f.driver.plugged_slots().is_empty());
    // Ao ser desplugado o controle ficou neutro.
    assert_eq!(f.driver.last_report(slot(0)), Some(XReport::default()));
}

#[test]
fn expired_slot_is_reusable_by_another_device_via_assign() {
    let f = fixture();
    for n in 1..=4 {
        f.manager.assign(&device(n), u32::from(n), None).unwrap();
    }
    f.manager.suspend(2);
    assert_eq!(
        f.manager.assign(&device(5), 5, None),
        Err(SlotError::NoSlot),
        "a reserva ainda vale"
    );
    f.clock.advance(Duration::from_secs(30));
    assert_eq!(f.manager.assign(&device(5), 5, None), Ok(slot(1)));
}

#[test]
fn same_device_reconnecting_over_a_live_session_takes_over_the_slot() {
    let f = fixture();
    f.manager.assign(&device(1), 7, None).unwrap();
    f.manager.submit(7, &PRESSED).unwrap();
    // O servidor ainda não percebeu a queda da conexão antiga.
    assert_eq!(f.manager.assign(&device(1), 8, None), Ok(slot(0)));
    assert_eq!(f.driver.plugged_slots(), [slot(0)]);
    assert_eq!(
        f.manager.submit(7, &PRESSED),
        Err(SlotError::UnknownSession)
    );
    f.manager.submit(8, &PRESSED).unwrap();
}

// ------------------------------------------------------------ liberação

#[test]
fn release_frees_the_slot_immediately() {
    let f = fixture();
    f.manager.assign(&device(1), 7, None).unwrap();
    f.manager.submit(7, &PRESSED).unwrap();
    assert_eq!(f.manager.release(7), Some(slot(0)));
    assert_eq!(state(&f, 0), SlotView::Free);
    assert!(f.driver.plugged_slots().is_empty());
    assert_eq!(f.driver.last_report(slot(0)), Some(XReport::default()));
    assert_eq!(f.manager.release(7), None);
}

#[test]
fn release_of_a_reserved_slot_never_touches_the_driver() {
    let f = fixture();
    f.manager.assign(&device(1), 7, None).unwrap();
    assert_eq!(f.manager.release(7), Some(slot(0)));
    assert!(f.driver.reports().is_empty());
}

// ------------------------------------------------------------ ações da UI

#[test]
fn disconnect_slot_returns_the_live_session() {
    let f = fixture();
    f.manager.assign(&device(1), 7, None).unwrap();
    f.manager.submit(7, &PRESSED).unwrap();
    assert_eq!(f.manager.disconnect_slot(slot(0)), Ok(Some(7)));
    assert_eq!(state(&f, 0), SlotView::Free);
    assert!(f.driver.plugged_slots().is_empty());
    assert_eq!(
        f.manager.disconnect_slot(slot(0)),
        Err(SlotError::EmptySlot)
    );
}

#[test]
fn disconnecting_a_suspended_slot_has_no_session() {
    let f = fixture();
    f.manager.assign(&device(1), 7, None).unwrap();
    f.manager.submit(7, &PRESSED).unwrap();
    f.manager.suspend(7);
    assert_eq!(f.manager.disconnect_slot(slot(0)), Ok(None));
    assert!(f.driver.plugged_slots().is_empty());
}

#[test]
fn pause_ignores_input_and_leaves_the_pad_neutral() {
    let f = fixture();
    f.manager.assign(&device(1), 7, None).unwrap();
    f.manager.submit(7, &PRESSED).unwrap();

    f.manager.set_paused(slot(0), true).unwrap();
    assert_eq!(f.driver.last_report(slot(0)), Some(XReport::default()));
    f.manager.submit(7, &PRESSED).unwrap();
    assert_eq!(f.driver.last_report(slot(0)), Some(XReport::default()));
    assert!(f.manager.snapshot().slots[0].paused);

    f.manager.set_paused(slot(0), false).unwrap();
    f.manager.submit(7, &PRESSED).unwrap();
    assert_eq!(f.driver.last_report(slot(0)), Some(PRESSED));
}

#[test]
fn pausing_a_reserved_slot_defers_the_plug() {
    let f = fixture();
    f.manager.assign(&device(1), 7, None).unwrap();
    f.manager.set_paused(slot(0), true).unwrap();
    f.manager.submit(7, &PRESSED).unwrap();
    assert!(f.driver.plugged_slots().is_empty());
    f.manager.set_paused(slot(0), false).unwrap();
    f.manager.submit(7, &PRESSED).unwrap();
    assert_eq!(f.driver.plugged_slots(), [slot(0)]);
}

#[test]
fn pausing_an_empty_slot_is_an_error() {
    let f = fixture();
    assert_eq!(
        f.manager.set_paused(slot(3), true),
        Err(SlotError::EmptySlot)
    );
}

#[test]
fn swap_exchanges_devices_and_replugs_active_pads() {
    let f = fixture();
    f.manager.assign(&device(1), 1, None).unwrap();
    f.manager.assign(&device(2), 2, None).unwrap();
    f.manager.submit(1, &PRESSED).unwrap();
    f.manager.submit(2, &PRESSED).unwrap();

    f.manager.swap(slot(0), slot(1)).unwrap();
    let snap = f.manager.snapshot();
    assert_eq!(snap.slots[0].device, Some(device(2)));
    assert_eq!(snap.slots[1].device, Some(device(1)));
    assert_eq!(snap.slots[0].state, SlotView::Active);
    assert_eq!(snap.slots[1].state, SlotView::Active);
    assert_eq!(f.driver.plugged_slots(), [slot(0), slot(1)]);

    // A sessão 2 agora comanda o slot 0.
    let only_b = XReport {
        buttons: xbuttons::B,
        ..XReport::default()
    };
    f.manager.submit(2, &only_b).unwrap();
    assert_eq!(f.driver.last_report(slot(0)), Some(only_b));
}

#[test]
fn swap_with_a_free_slot_moves_the_device() {
    let f = fixture();
    f.manager.assign(&device(1), 1, None).unwrap();
    f.manager.submit(1, &PRESSED).unwrap();
    f.manager.swap(slot(0), slot(3)).unwrap();
    assert_eq!(state(&f, 0), SlotView::Free);
    assert_eq!(state(&f, 3), SlotView::Active);
    assert_eq!(f.driver.plugged_slots(), [slot(3)]);
    assert_eq!(f.manager.swap(slot(0), slot(2)), Err(SlotError::EmptySlot));
}

#[test]
fn swap_keeps_a_suspended_reservation_and_its_clock() {
    let f = fixture();
    f.manager.assign(&device(1), 1, None).unwrap();
    f.manager.submit(1, &PRESSED).unwrap();
    f.manager.suspend(1);
    f.clock.advance(Duration::from_secs(10));
    f.manager.swap(slot(0), slot(2)).unwrap();
    assert!(f.driver.plugged_slots().is_empty());
    assert!(matches!(state(&f, 2), SlotView::Suspended { .. }));

    // O relógio da reserva não reinicia.
    f.clock.advance(Duration::from_secs(20));
    assert_eq!(f.manager.tick(), [slot(2)]);
}

#[test]
fn swap_falls_back_to_reserved_when_replug_fails_and_recovers_on_input() {
    let f = fixture();
    f.manager.assign(&device(1), 1, None).unwrap();
    f.manager.submit(1, &PRESSED).unwrap();
    f.driver.set_status(DriverStatus::Error("caiu".into()));
    f.manager.swap(slot(0), slot(1)).unwrap();
    assert_eq!(state(&f, 1), SlotView::Reserved);

    f.driver.set_status(DriverStatus::Ready);
    f.manager.submit(1, &PRESSED).unwrap();
    assert_eq!(state(&f, 1), SlotView::Active);
    assert_eq!(f.driver.plugged_slots(), [slot(1)]);
}

// ------------------------------------------------------------ snapshot

#[test]
fn snapshot_reports_state_session_and_expiry() {
    let f = fixture();
    f.manager.assign(&device(1), 7, None).unwrap();
    f.manager.submit(7, &PRESSED).unwrap();
    let snap = f.manager.snapshot();
    assert_eq!(snap.slots[0].session, Some(7));
    assert_eq!(snap.slots[0].user_index, Some(0));
    assert_eq!(snap.free_count(), 3);

    f.manager.suspend(7);
    let snap = f.manager.snapshot();
    let SlotView::Suspended { expires_at } = snap.slots[0].state else {
        panic!("esperava Suspended");
    };
    assert_eq!(expires_at, f.clock.now() + Duration::from_secs(30));
    assert_eq!(snap.slots[0].session, None);
}

#[test]
fn subscribers_receive_a_snapshot_per_change() {
    let f = fixture();
    let rx = f.manager.subscribe();
    f.manager.assign(&device(1), 7, None).unwrap();
    assert_eq!(rx.try_recv().unwrap().slots[0].state, SlotView::Reserved);
    f.manager.submit(7, &PRESSED).unwrap();
    assert_eq!(rx.try_recv().unwrap().slots[0].state, SlotView::Active);
    f.manager.release(7);
    assert_eq!(rx.try_recv().unwrap().slots[0].state, SlotView::Free);
    assert!(
        rx.try_recv().is_err(),
        "submit no hot path não publica nada"
    );

    // Um assinante que sumiu não atrapalha.
    drop(rx);
    f.manager.assign(&device(1), 8, None).unwrap();
}

#[test]
fn every_slot_can_be_used_at_once() {
    let f = fixture();
    for n in 1..=4u8 {
        f.manager.assign(&device(n), u32::from(n), None).unwrap();
        let report = XReport {
            lx: i16::from(n),
            ..XReport::default()
        };
        f.manager.submit(u32::from(n), &report).unwrap();
    }
    for n in 1..=4u8 {
        let last = f.driver.last_report(slot(n - 1)).unwrap();
        assert_eq!(last.lx, i16::from(n));
    }
}

// ------------------------------------------------------------ monitor ao vivo

#[test]
fn live_report_mirrors_the_last_state_sent_to_the_pad() {
    let f = fixture();
    assert_eq!(f.manager.live_report(slot(0)), None, "sem controle plugado");
    f.manager.assign(&device(1), 7, None).unwrap();
    assert_eq!(
        f.manager.live_report(slot(0)),
        None,
        "reservado ainda não plugou"
    );

    let report = XReport {
        buttons: 0x1234,
        lt: 200,
        rt: 17,
        lx: -32768,
        ly: 32767,
        rx: -1,
        ry: 300,
    };
    f.manager.submit(7, &report).unwrap();
    assert_eq!(f.manager.live_report(slot(0)), Some(report));
    assert_eq!(f.manager.live_report(slot(1)), None);

    // O neutro do watchdog também aparece.
    assert!(f.manager.neutralize_session(7));
    assert_eq!(f.manager.live_report(slot(0)), Some(XReport::default()));

    // Sem controle plugado de novo depois de liberar.
    f.manager.release(7);
    assert_eq!(f.manager.live_report(slot(0)), None);
}

#[test]
fn the_reconnect_window_can_change_while_running() {
    let f = fixture();
    assert_eq!(f.manager.reconnect_window(), Duration::from_secs(30));
    f.manager.assign(&device(1), 1, None).unwrap();
    f.manager.submit(1, &PRESSED).unwrap();
    f.manager.suspend(1);

    // Com 60 s a reserva de 30 s ainda vale; a foto já mostra o prazo novo.
    f.manager.set_reconnect_window(Duration::from_secs(60));
    f.clock.advance(Duration::from_secs(40));
    assert!(f.manager.tick().is_empty());
    assert!(matches!(state(&f, 0), SlotView::Suspended { .. }));

    // Voltando para 10 s, a reserva (que já tem 40 s) vence na próxima conferência.
    f.manager.set_reconnect_window(Duration::from_secs(10));
    assert_eq!(f.manager.tick(), [slot(0)]);
    assert_eq!(state(&f, 0), SlotView::Free);
}
