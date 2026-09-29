//! Testes com o `ViGEmBus` real. Precisam do driver instalado e mexem nos controles `XInput`
//! da máquina, então ficam ignorados por padrão:
//!
//! ```text
//! cargo test -p padgaming-driver --test vigem_hardware -- --ignored --test-threads=1
//! ```
#![cfg(windows)]

use std::time::{Duration, Instant};

use padgaming_driver::{DriverStatus, SlotId, VigemDriver, VirtualPadDriver, XReport, xbuttons};
use rusty_xinput::XInputHandle;

fn slot(i: u8) -> SlotId {
    SlotId::new(i).unwrap()
}

fn driver() -> VigemDriver {
    let driver = VigemDriver::new();
    assert_eq!(
        driver.status(),
        DriverStatus::Ready,
        "ViGEmBus não está pronto"
    );
    driver
}

/// Procura o índice `XInput` cujo estado bate com `report` (há controles físicos na máquina).
fn find_xinput(xinput: &XInputHandle, report: &XReport) -> Option<u32> {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        for index in 0..4 {
            if let Ok(state) = xinput.get_state(index) {
                let g = state.raw.Gamepad;
                if g.wButtons == report.buttons
                    && g.bLeftTrigger == report.lt
                    && g.bRightTrigger == report.rt
                    && g.sThumbLX == report.lx
                    && g.sThumbLY == report.ly
                    && g.sThumbRX == report.rx
                    && g.sThumbRY == report.ry
                {
                    return Some(index);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

const REPORT: XReport = XReport {
    buttons: xbuttons::A | xbuttons::Y | xbuttons::LEFT_SHOULDER | xbuttons::DPAD_UP,
    lt: 77,
    rt: 200,
    lx: 12345,
    ly: -23456,
    rx: 3333,
    ry: -4444,
};

#[test]
#[ignore = "requer ViGEmBus instalado"]
fn report_reaches_xinput_and_unplug_removes_the_pad() {
    let xinput = XInputHandle::load_default().expect("XInput indisponível");
    let driver = driver();
    let pad = driver.plug(slot(0)).unwrap();

    // O controle novo só aceita updates depois de pronto; repete até o XInput enxergar.
    let deadline = Instant::now() + Duration::from_secs(3);
    let index = loop {
        driver.update(&pad, &REPORT).unwrap();
        if let Some(i) = find_xinput(&xinput, &REPORT) {
            break i;
        }
        assert!(
            Instant::now() < deadline,
            "o XInput nunca viu o controle virtual"
        );
    };

    // O índice medido no plug tem de ser o que o XInput realmente usa.
    assert_eq!(driver.user_index(&pad), Some(u8::try_from(index).unwrap()));

    driver.unplug(pad).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        xinput
            .get_state(index)
            .map_or(true, |s| s.raw.Gamepad.wButtons != REPORT.buttons),
        "o controle continuou visível depois do unplug"
    );
}

#[test]
#[ignore = "requer ViGEmBus instalado"]
fn all_four_slots_plug_and_a_fifth_replug_is_rejected() {
    let driver = driver();
    let pads: Vec<_> = SlotId::all().map(|s| driver.plug(s).unwrap()).collect();
    assert!(driver.plug(slot(2)).is_err(), "slot ocupado deveria falhar");
    for pad in pads {
        driver.unplug(pad).unwrap();
    }
    // Depois de liberar, o slot volta a funcionar.
    let again = driver.plug(slot(2)).unwrap();
    driver.unplug(again).unwrap();
}

#[test]
#[ignore = "requer ViGEmBus instalado"]
fn rumble_from_a_game_arrives_as_rumble_event() {
    let xinput = XInputHandle::load_default().expect("XInput indisponível");
    let driver = driver();
    let rumble = driver.rumble_events();
    let pad = driver.plug(slot(1)).unwrap();

    let deadline = Instant::now() + Duration::from_secs(3);
    let index = loop {
        driver.update(&pad, &REPORT).unwrap();
        if let Some(i) = find_xinput(&xinput, &REPORT) {
            break i;
        }
        assert!(
            Instant::now() < deadline,
            "o XInput nunca viu o controle virtual"
        );
    };

    // Dá tempo da thread de notificação armar o pedido.
    std::thread::sleep(Duration::from_millis(200));
    xinput.set_state(index, 0x8000, 0x4000).unwrap();
    // O driver também manda notificações zeradas (estado inicial); espera a que importa.
    let deadline = Instant::now() + Duration::from_secs(2);
    let event = loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let event = rumble
            .recv_timeout(left)
            .expect("nenhum RumbleEvent com vibração chegou");
        if (event.large, event.small) != (0, 0) {
            break event;
        }
    };
    assert_eq!(event.slot, slot(1));
    assert_eq!((event.large, event.small), (0x80, 0x40));

    xinput.set_state(index, 0, 0).unwrap();
    driver.unplug(pad).unwrap();
}

#[test]
#[ignore = "requer ViGEmBus instalado"]
fn dropping_the_driver_removes_the_pads() {
    let xinput = XInputHandle::load_default().expect("XInput indisponível");
    let driver = driver();
    let pad = driver.plug(slot(0)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let index = loop {
        driver.update(&pad, &REPORT).unwrap();
        if let Some(i) = find_xinput(&xinput, &REPORT) {
            break i;
        }
        assert!(
            Instant::now() < deadline,
            "o XInput nunca viu o controle virtual"
        );
    };
    drop(driver);
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        xinput
            .get_state(index)
            .map_or(true, |s| s.raw.Gamepad.wButtons != REPORT.buttons),
        "o controle ficou 'fantasma' depois do drop"
    );
}

#[test]
#[ignore = "requer ViGEmBus instalado"]
fn update_cost_is_small() {
    let driver = driver();
    let pad = driver.plug(slot(3)).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    driver.update(&pad, &REPORT).unwrap();

    let n = 2000u32;
    let start = Instant::now();
    for i in 0..n {
        let report = XReport {
            lx: i16::try_from(i % 1000).unwrap(),
            ..REPORT
        };
        driver.update(&pad, &report).unwrap();
    }
    let per_call = start.elapsed() / n;
    println!("update(): {per_call:?} por chamada");
    driver.unplug(pad).unwrap();
    assert!(
        per_call < Duration::from_micros(500),
        "update() lento: {per_call:?}"
    );
}

#[test]
#[ignore = "requer ViGEmBus instalado"]
fn status_is_ready_and_service_is_running() {
    use padgaming_driver::setup::{self, ServiceState};

    let driver = VigemDriver::new();
    assert_eq!(driver.status(), DriverStatus::Ready);
    assert_eq!(setup::service_state(), ServiceState::Running);
    let version = setup::installed_version().expect("versão no registro");
    println!("ViGEmBus instalado: {version}");
}

#[test]
#[ignore = "requer ViGEmBus instalado"]
fn plugged_pads_are_seen_as_occupied_xinput_slots() {
    use padgaming_driver::xinput::XInputProbe;

    let probe = XInputProbe::load().expect("XInput indisponível");
    let driver = driver();
    let before = probe.connected();
    let pad = driver.plug(slot(0)).unwrap();
    let index = usize::from(driver.user_index(&pad).expect("índice do controle novo"));
    assert!(
        !before[index],
        "o índice devolvido já estava ocupado antes do plug"
    );
    assert!(
        probe.connected()[index],
        "o controle novo não aparece como ocupado"
    );
    driver.unplug(pad).unwrap();
}
