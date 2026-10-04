//! O simulador contra o server de input de verdade (loopback + `MockDriver`).
#![cfg(debug_assertions)] // a sessão de desenvolvimento só existe em builds de debug

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use vynpad_core::SlotManager;
use vynpad_driver::{MockDriver, SlotId};
use vynpad_mapping::{CompiledProfile, ProfileCell};
use vynpad_net::input::{DEV_KEY, DEV_SESSION_ID, InputConfig, InputServer, SessionSpec};
use vynpad_protocol::Secret;
use vynpad_protocol::control::Uuid;
use vynpad_sim::client::{self, ClientConfig, Event};
use vynpad_sim::link::FaultConfig;
use vynpad_sim::report::Percentiles;
use vynpad_sim::source::{Pattern, PatternKind, Script};

struct Rig {
    server: InputServer,
    driver: Arc<MockDriver>,
}

fn rig(sessions: u32) -> Rig {
    let driver = Arc::new(MockDriver::default());
    let slots = Arc::new(SlotManager::new(driver.clone()));
    let server = InputServer::start(
        InputConfig {
            bind: "127.0.0.1:0".parse().unwrap(),
            poll_interval: Duration::from_millis(20),
            ..InputConfig::default()
        },
        slots.clone(),
    )
    .unwrap();
    let cell = Arc::new(ProfileCell::new(CompiledProfile::identity()));
    for i in 0..sessions {
        let id = DEV_SESSION_ID + i;
        let device = Uuid::parse(&format!("00000000-0000-4000-8000-00000000000{i}")).unwrap();
        slots.assign(&device, id, None).unwrap();
        server.add_session(SessionSpec {
            session_id: id,
            key: Secret::new(DEV_KEY),
            profile: cell.clone(),
        });
    }
    Rig { server, driver }
}

fn config(rig: &Rig, fault: FaultConfig) -> ClientConfig {
    ClientConfig {
        server: rig.server.local_addr(),
        session_id: DEV_SESSION_ID,
        key: DEV_KEY,
        rate_hz: 250,
        fault,
        seed: 1,
    }
}

fn never() -> AtomicBool {
    AtomicBool::new(false)
}

#[test]
fn the_simulator_and_the_server_agree_on_the_dev_session() {
    assert_eq!(vynpad_sim::DEV_SESSION_ID, DEV_SESSION_ID);
    assert_eq!(vynpad_sim::DEV_KEY, DEV_KEY);
}

#[test]
fn a_circle_pattern_moves_the_virtual_pad_and_pings_come_back() {
    let rig = rig(1);
    let mut source = Pattern::new(PatternKind::Circle, 1);
    let report = client::run(
        &config(&rig, FaultConfig::default()),
        &mut source,
        &never(),
        Some(Duration::from_millis(1200)),
        &mut |_| {},
    )
    .unwrap();

    assert!(
        (280..=310).contains(&report.inputs_sent),
        "{}",
        report.inputs_sent
    );
    assert!(report.pongs >= 4, "{report:?}");
    assert_eq!(report.bad_incoming, 0);
    let rtt = Percentiles::of(&report.rtt_us);
    assert!(
        rtt.p50 < 20_000,
        "RTT de loopback deveria ser baixo: {rtt:?}"
    );

    let stats = rig.server.stats().snapshot();
    assert_eq!(stats.dropped(), 0);
    assert!(
        stats.inputs >= report.inputs_sent - 5,
        "{stats:?} vs {}",
        report.inputs_sent
    );
    // O analógico chegou ao pad virtual desenhando o círculo.
    let lxs: Vec<i16> = rig.driver.reports().iter().map(|(_, r)| r.lx).collect();
    assert!(lxs.iter().any(|x| *x > 15_000) && lxs.iter().any(|x| *x < -15_000));
    let last = rig.driver.last_report(SlotId::new(0).unwrap()).unwrap();
    assert!(last.lt > 0 || last.rt > 0);
}

#[test]
fn a_script_drives_buttons_and_ends_on_its_own() {
    let rig = rig(1);
    let mut script = Script::parse(
        r#"
        [[step]]
        duration = "300ms"
        buttons = ["a", "start"]
        lx = 0.5
    "#,
    )
    .unwrap();
    let report = client::run(
        &config(&rig, FaultConfig::default()),
        &mut script,
        &never(),
        None, // sem limite: quem encerra é o fim do script
        &mut |_| {},
    )
    .unwrap();
    assert!(
        report.elapsed < Duration::from_secs(2),
        "{:?}",
        report.elapsed
    );
    let last = rig.driver.last_report(SlotId::new(0).unwrap()).unwrap();
    assert_eq!(
        last.buttons,
        vynpad_driver::xbuttons::A | vynpad_driver::xbuttons::START
    );
    assert_eq!(last.lx, 16383);
}

#[test]
fn total_loss_delivers_nothing_and_is_reported() {
    let rig = rig(1);
    let mut source = Pattern::new(PatternKind::Stress, 3);
    let report = client::run(
        &config(
            &rig,
            FaultConfig {
                loss: 1.0,
                ..FaultConfig::default()
            },
        ),
        &mut source,
        &never(),
        Some(Duration::from_millis(500)),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(report.pongs, 0);
    assert_eq!(report.lost_by_fault, report.inputs_sent + report.pings_sent);
    assert_eq!(rig.server.stats().snapshot().received, 0);
}

#[test]
fn simulated_delay_shows_up_in_the_rtt() {
    let rig = rig(1);
    let mut source = Pattern::new(PatternKind::Circle, 1);
    let fault = FaultConfig {
        delay: Duration::from_millis(30),
        ..FaultConfig::default()
    };
    let report = client::run(
        &config(&rig, fault),
        &mut source,
        &never(),
        Some(Duration::from_millis(1000)),
        &mut |_| {},
    )
    .unwrap();
    let rtt = Percentiles::of(&report.rtt_us);
    assert!(report.pongs >= 3);
    assert!(
        rtt.p50 >= 30_000,
        "o atraso de 30 ms deveria aparecer no RTT: {rtt:?}"
    );
    assert!(rtt.p50 < 80_000, "{rtt:?}");
}

#[test]
fn reordered_packets_are_dropped_by_the_server_as_replays() {
    let rig = rig(1);
    let mut source = Pattern::new(PatternKind::Stress, 5);
    let fault = FaultConfig {
        reorder: true,
        ..FaultConfig::default()
    };
    client::run(
        &config(&rig, fault),
        &mut source,
        &never(),
        Some(Duration::from_millis(1000)),
        &mut |_| {},
    )
    .unwrap();
    let stats = rig.server.stats().snapshot();
    assert!(stats.dropped_replay > 5, "{stats:?}");
    assert!(stats.inputs > 100, "{stats:?}");
    assert_eq!(stats.dropped_auth, 0);
}

#[test]
fn the_stop_flag_ends_the_run_promptly() {
    let rig = rig(1);
    let stop = AtomicBool::new(true);
    let mut source = Pattern::new(PatternKind::Circle, 1);
    let mut events = Vec::new();
    let report = client::run(
        &config(&rig, FaultConfig::default()),
        &mut source,
        &stop,
        None,
        &mut |e: Event| events.push(e),
    )
    .unwrap();
    assert_eq!(report.inputs_sent, 0);
}

#[test]
fn zero_rate_is_refused() {
    let rig = rig(1);
    let mut cfg = config(&rig, FaultConfig::default());
    cfg.rate_hz = 0;
    let mut source = Pattern::new(PatternKind::Circle, 1);
    assert!(client::run(&cfg, &mut source, &never(), None, &mut |_| {}).is_err());
}
