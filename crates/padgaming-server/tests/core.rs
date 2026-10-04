//! O `Core` inteiro (banco em arquivo, TLS, UDP e slots reais, `MockDriver`), com o `padgaming-sim`
//! no papel do celular.
#![allow(clippy::float_cmp)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use padgaming_driver::{MockDriver, SlotId};
use padgaming_mapping::Profile;
use padgaming_net::control::{ControlConfig, ControlEvent};
use padgaming_protocol::Secret;
use padgaming_protocol::control::{Auth, ByeReason, Bytes, ControlMessage, RejectReason, Uuid};
use padgaming_protocol::pairing_uri::PairingUri;
use padgaming_server::{ApprovalEvent, Core, CoreConfig, CoreEvent, Quality};
use padgaming_sim::client::{self, ClientConfig};
use padgaming_sim::control::{self, ControlSession, Identity, Outcome, Trust};
use padgaming_sim::link::FaultConfig;
use padgaming_sim::source::Script;
use padgaming_storage::{DeviceDefaults, LogQuery, Ports, ProfileInput};
use tokio::sync::broadcast;

struct Rig {
    core: Core,
    driver: Arc<MockDriver>,
    dir: tempfile::TempDir,
    events: broadcast::Receiver<CoreEvent>,
}

fn config(dir: &std::path::Path, driver: &Arc<MockDriver>) -> CoreConfig {
    let mut config = CoreConfig::new(dir);
    config.driver = Some(driver.clone());
    config.server_name = Some("PC de teste".into());
    config.bind_ip = "127.0.0.1".parse().unwrap();
    config.ports_override = Some(Ports {
        control: 0,
        input: 0,
        discovery: 0,
    });
    config.mdns = false;
    config.control = ControlConfig {
        ping_after: Duration::from_millis(100),
        // O client de teste não responde ao keepalive do TLS.
        dead_after: Duration::from_secs(60),
        hello_timeout: Duration::from_secs(3),
        approval_timeout: Duration::from_secs(2),
        ..ControlConfig::default()
    };
    config
}

async fn rig() -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let driver = Arc::new(MockDriver::default());
    let core = Core::start(config(dir.path(), &driver)).await.unwrap();
    let events = core.subscribe();
    Rig {
        core,
        driver,
        dir,
        events,
    }
}

impl Rig {
    fn control_addr(&self) -> SocketAddr {
        SocketAddr::new("127.0.0.1".parse().unwrap(), self.core.ports().control)
    }

    /// Reinicia o server na mesma pasta de dados (o que acontece quando o app fecha e abre).
    async fn restart(self) -> Self {
        let Self {
            core, driver, dir, ..
        } = self;
        core.shutdown().await;
        let core = Core::start(config(dir.path(), &driver)).await.unwrap();
        let events = core.subscribe();
        Self {
            core,
            driver,
            dir,
            events,
        }
    }
}

fn who(n: u8) -> Identity {
    Identity::simulated(Uuid::parse(&format!("00000000-0000-4000-8000-0000000000{n:02}")).unwrap())
}

fn token(core: &Core) -> Auth {
    Auth::PairingToken {
        token: Secret::new(Bytes(*core.pairing().issue_pairing_token().expose())),
    }
}

async fn connect(addr: SocketAddr, trust: Trust, who: Identity, auth: Auth) -> Outcome {
    tokio::task::spawn_blocking(move || control::connect(addr, trust, &who, auth).unwrap())
        .await
        .unwrap()
}

async fn welcomed(rig: &Rig, n: u8) -> Box<ControlSession> {
    match connect(
        rig.control_addr(),
        Trust::FirstUse,
        who(n),
        token(&rig.core),
    )
    .await
    {
        Outcome::Welcome(session) => session,
        Outcome::Rejected(r) => panic!("recusado: {r:?}"),
    }
}

async fn eventually(what: &str, mut cond: impl FnMut() -> bool) {
    for _ in 0..300 {
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("não aconteceu: {what}");
}

async fn eventually_async<F: std::future::Future<Output = bool>>(
    what: &str,
    mut cond: impl FnMut() -> F,
) {
    for _ in 0..300 {
        if cond().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("não aconteceu: {what}");
}

async fn next_matching<T>(
    rx: &mut broadcast::Receiver<CoreEvent>,
    mut pick: impl FnMut(&CoreEvent) -> Option<T>,
) -> T {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let event = tokio::time::timeout(left, rx.recv())
            .await
            .expect("o evento não chegou")
            .unwrap();
        if let Some(found) = pick(&event) {
            return found;
        }
    }
}

/// Segura o analógico esquerdo em `lx` (fração do curso) até `stop`.
fn hold(
    rig: &Rig,
    session: &ControlSession,
    lx: f32,
    stop: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    let w = &session.welcome;
    let config = ClientConfig {
        server: SocketAddr::new("127.0.0.1".parse().unwrap(), w.udp_port),
        session_id: w.session_id,
        key: w.udp_key.expose().0,
        rate_hz: 250,
        fault: FaultConfig::default(),
        seed: 1,
    };
    let _ = rig;
    std::thread::spawn(move || {
        let mut script =
            Script::parse(&format!("[[step]]\nduration = \"60s\"\nlx = {lx}\n")).unwrap();
        client::run(&config, &mut script, &stop, None, &mut |_| {}).unwrap();
    })
}

fn lx(rig: &Rig, slot: u8) -> i16 {
    rig.driver
        .last_report(SlotId::new(slot).unwrap())
        .map_or(0, |r| r.lx)
}

fn profile_input(name: &str, edit: impl FnOnce(&mut Profile)) -> ProfileInput {
    let mut profile = Profile::identity();
    edit(&mut profile);
    ProfileInput {
        name: name.into(),
        description: None,
        icon: None,
        profile,
    }
}

// ------------------------------------------------------------------ pareamento persistente

#[tokio::test(flavor = "multi_thread")]
async fn a_paired_device_survives_a_restart_and_reconnects_with_the_pinned_certificate() {
    let rig = rig().await;
    let first = welcomed(&rig, 1).await;
    let device_token = *first.welcome.device_token.clone().unwrap().expose();
    let fingerprint = first.fingerprint;
    assert_eq!(fingerprint, rig.core.fingerprint());
    first.drop_connection();

    let stored = rig
        .core
        .devices()
        .find(who(1).device_id.as_str())
        .await
        .unwrap();
    let stored = stored.expect("o pareamento foi gravado no banco");
    assert!(!stored.revoked);
    assert_eq!(stored.name, "PadGaming Sim");
    assert_eq!(stored.os_version.as_deref(), Some(std::env::consts::OS));

    // O app fecha e abre de novo: mesma pasta de dados, portas novas.
    let rig = rig.restart().await;
    assert_eq!(
        rig.core.fingerprint(),
        fingerprint,
        "a identidade TLS persiste"
    );
    let again = match connect(
        rig.control_addr(),
        Trust::Pinned(fingerprint),
        who(1),
        Auth::DeviceToken {
            token: Secret::new(device_token),
        },
    )
    .await
    {
        Outcome::Welcome(session) => session,
        Outcome::Rejected(r) => panic!("recusado depois de reiniciar: {r:?}"),
    };
    assert!(
        again.welcome.device_token.is_none(),
        "não é um pareamento novo"
    );
    assert_eq!(again.welcome.config.slot, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn connections_disconnections_and_rejections_go_to_the_history() {
    let mut rig = rig().await;
    let session = welcomed(&rig, 1).await;
    next_matching(&mut rig.events, |e| {
        matches!(e, CoreEvent::Control(ControlEvent::SessionStarted { .. })).then_some(())
    })
    .await;
    session.drop_connection();
    next_matching(&mut rig.events, |e| {
        matches!(e, CoreEvent::Control(ControlEvent::SessionEnded { .. })).then_some(())
    })
    .await;
    // Um desconhecido com credencial errada.
    let bad = Auth::PairingToken {
        token: Secret::new(Bytes([9; 16])),
    };
    let rejected = connect(rig.control_addr(), Trust::FirstUse, who(7), bad).await;
    assert!(matches!(rejected, Outcome::Rejected(r) if r.reason == RejectReason::BadAuth));
    next_matching(&mut rig.events, |e| {
        matches!(e, CoreEvent::Control(ControlEvent::Rejected { .. })).then_some(())
    })
    .await;

    // O pump grava em seguida ao evento.
    eventually_async("histórico completo", || async {
        rig.core
            .log()
            .list(&LogQuery::default())
            .await
            .unwrap()
            .len()
            >= 4
    })
    .await;
    let log = rig.core.log().list(&LogQuery::default()).await.unwrap();
    let events: Vec<(&str, Option<&str>)> = log
        .iter()
        .rev()
        .map(|e| (e.event.as_str(), e.device_id.as_deref()))
        .collect();
    let id = who(1).device_id;
    assert_eq!(
        events,
        [
            ("paired", Some(id.as_str())),
            ("connected", Some(id.as_str())),
            ("disconnected", Some(id.as_str())),
            ("rejected", None), // dispositivo desconhecido: sem chave estrangeira
        ]
    );
    let connected = log.iter().find(|e| e.event == "connected").unwrap();
    assert_eq!(connected.slot, Some(0));
    assert_eq!(connected.detail.as_deref(), Some("127.0.0.1"));

    let device = rig.core.devices().find(id.as_str()).await.unwrap().unwrap();
    assert_eq!(device.last_ip.as_deref(), Some("127.0.0.1"));
    assert!(device.last_seen_at.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn revoking_a_connected_device_ends_its_session_and_blocks_it() {
    let rig = rig().await;
    let session = welcomed(&rig, 1).await;
    let device_token = *session.welcome.device_token.clone().unwrap().expose();
    let id = who(1).device_id;

    rig.core.revoke_device(id.as_str()).await.unwrap();
    let message = tokio::task::spawn_blocking(move || {
        let mut session = session;
        loop {
            match session.recv(Duration::from_secs(3)).unwrap() {
                ControlMessage::Ping => {}
                other => return other,
            }
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        message,
        ControlMessage::Bye(b) if b.reason == ByeReason::Revoked
    ));
    // O slot foi liberado e o token deixou de valer.
    eventually("slot livre", || {
        rig.core.slots().snapshot().free_count() == 4
    })
    .await;
    let auth = Auth::DeviceToken {
        token: Secret::new(device_token),
    };
    let outcome = connect(rig.control_addr(), Trust::FirstUse, who(1), auth).await;
    assert!(matches!(outcome, Outcome::Rejected(r) if r.reason == RejectReason::Revoked));
    assert!(
        rig.core
            .devices()
            .find(id.as_str())
            .await
            .unwrap()
            .unwrap()
            .revoked
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn forgetting_a_device_makes_it_pair_again_from_scratch() {
    let rig = rig().await;
    let session = welcomed(&rig, 1).await;
    let device_token = *session.welcome.device_token.clone().unwrap().expose();
    session.drop_connection();
    rig.core
        .forget_device(who(1).device_id.as_str())
        .await
        .unwrap();
    assert!(
        rig.core
            .devices()
            .find(who(1).device_id.as_str())
            .await
            .unwrap()
            .is_none()
    );

    let auth = Auth::DeviceToken {
        token: Secret::new(device_token),
    };
    let outcome = connect(rig.control_addr(), Trust::FirstUse, who(1), auth).await;
    assert!(matches!(outcome, Outcome::Rejected(r) if r.reason == RejectReason::BadAuth));
    // Com um QR novo, pareia normalmente.
    welcomed(&rig, 1).await;
}

// ------------------------------------------------------------------ slots e perfis

#[tokio::test(flavor = "multi_thread")]
async fn the_preferred_slot_and_default_profile_are_used_on_connect() {
    let rig = rig().await;
    let first = welcomed(&rig, 1).await;
    first.drop_connection();
    let id = who(1).device_id;
    let mirrored = rig
        .core
        .profiles()
        .create(profile_input("Espelhado", |p| {
            p.sticks.left.invert_x = true;
        }))
        .await
        .unwrap();
    rig.core
        .set_device_defaults(
            id.as_str(),
            &DeviceDefaults {
                preferred_slot: Some(2),
                default_profile_id: Some(mirrored.summary.id.clone()),
                color: None,
            },
        )
        .await
        .unwrap();
    // A reserva do slot 0 (30 s) não atrapalha: o dispositivo volta ao slot em que já estava, e
    // a preferência vale para quem chega com o slot livre.
    rig.core.disconnect_slot(SlotId::new(0).unwrap()).unwrap();

    let auth = token(&rig.core);
    let Outcome::Welcome(session) =
        connect(rig.control_addr(), Trust::FirstUse, who(1), auth).await
    else {
        panic!("deveria conectar");
    };
    assert_eq!(session.welcome.config.slot, 2, "slot preferido");

    // O perfil padrão do dispositivo é o do slot: o analógico sai espelhado.
    let stop = Arc::new(AtomicBool::new(false));
    let client = hold(&rig, &session, 0.5, Arc::clone(&stop));
    eventually("analógico espelhado", || lx(&rig, 2) < -5_000).await;
    stop.store(true, Ordering::Relaxed);
    client.join().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn editing_a_profile_changes_the_active_slot_without_reconnecting() {
    let rig = rig().await;
    let session = welcomed(&rig, 1).await;
    let mirrored = rig
        .core
        .profiles()
        .create(profile_input("Espelhado", |p| {
            p.sticks.left.invert_x = true;
        }))
        .await
        .unwrap();
    let id = mirrored.summary.id.clone();
    rig.core
        .set_device_defaults(
            who(1).device_id.as_str(),
            &DeviceDefaults {
                default_profile_id: Some(id.clone()),
                ..DeviceDefaults::default()
            },
        )
        .await
        .unwrap();
    // Reconecta para o perfil novo valer (a preferência vale na conexão).
    session.drop_connection();
    let session = match connect(
        rig.control_addr(),
        Trust::FirstUse,
        who(1),
        token(&rig.core),
    )
    .await
    {
        Outcome::Welcome(s) => s,
        Outcome::Rejected(r) => panic!("{r:?}"),
    };

    let stop = Arc::new(AtomicBool::new(false));
    let client = hold(&rig, &session, 0.5, Arc::clone(&stop));
    eventually("espelhado", || lx(&rig, 0) < -5_000).await;
    let sessions_before = rig.core.control().session_count();

    // O usuário edita o perfil na UI: o slot muda na hora, com a mesma sessão.
    rig.core
        .profiles()
        .save(
            &id,
            profile_input("Espelhado", |p| p.sticks.left.invert_x = false),
        )
        .await
        .unwrap();
    eventually("normal de novo", || lx(&rig, 0) > 5_000).await;
    assert_eq!(
        rig.core.control().session_count(),
        sessions_before,
        "sem reconectar"
    );

    stop.store(true, Ordering::Relaxed);
    client.join().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn turning_the_gyro_on_in_a_profile_tells_the_client_to_send_motion() {
    let rig = rig().await;
    let session = welcomed(&rig, 1).await;
    assert!(!session.welcome.config.gyro_enabled);
    let with_gyro = rig
        .core
        .profiles()
        .create(profile_input("Com gyro", |p| p.gyro.enabled = true))
        .await
        .unwrap();
    rig.core
        .set_device_defaults(
            who(1).device_id.as_str(),
            &DeviceDefaults {
                default_profile_id: Some(with_gyro.summary.id.clone()),
                ..DeviceDefaults::default()
            },
        )
        .await
        .unwrap();
    session.drop_connection();
    let session = match connect(
        rig.control_addr(),
        Trust::FirstUse,
        who(1),
        token(&rig.core),
    )
    .await
    {
        Outcome::Welcome(s) => s,
        Outcome::Rejected(r) => panic!("{r:?}"),
    };
    assert!(
        session.welcome.config.gyro_enabled,
        "o welcome já traz o gyro do perfil"
    );

    // Desligar o gyro no perfil manda um `config` novo à sessão em andamento.
    rig.core
        .profiles()
        .save(
            &with_gyro.summary.id,
            profile_input("Com gyro", |p| p.gyro.enabled = false),
        )
        .await
        .unwrap();
    let config = tokio::task::spawn_blocking(move || {
        let mut session = session;
        loop {
            if let ControlMessage::Config(c) = session.recv(Duration::from_secs(3)).unwrap() {
                return c;
            }
        }
    })
    .await
    .unwrap();
    assert!(!config.gyro_enabled);
    assert_eq!(config.slot, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn deleting_a_profile_in_use_needs_confirmation_and_moves_the_slot_to_the_default() {
    let rig = rig().await;
    let session = welcomed(&rig, 1).await;
    let mirrored = rig
        .core
        .profiles()
        .create(profile_input("Espelhado", |p| {
            p.sticks.left.invert_x = true;
        }))
        .await
        .unwrap();
    let id = mirrored.summary.id.clone();
    rig.core
        .set_device_defaults(
            who(1).device_id.as_str(),
            &DeviceDefaults {
                default_profile_id: Some(id.clone()),
                ..DeviceDefaults::default()
            },
        )
        .await
        .unwrap();
    session.drop_connection();
    let session = match connect(
        rig.control_addr(),
        Trust::FirstUse,
        who(1),
        token(&rig.core),
    )
    .await
    {
        Outcome::Welcome(s) => s,
        Outcome::Rejected(r) => panic!("{r:?}"),
    };
    let stop = Arc::new(AtomicBool::new(false));
    let client = hold(&rig, &session, 0.5, Arc::clone(&stop));
    eventually("espelhado", || lx(&rig, 0) < -5_000).await;

    // Em uso por um controle ativo e por um dispositivo: pede confirmação.
    let error = rig.core.profiles().delete(&id, false).await.unwrap_err();
    assert!(
        matches!(
            error,
            padgaming_storage::DataError::InUse {
                devices: 1,
                slots: 1
            }
        ),
        "{error}"
    );
    // Confirmado: o slot volta ao perfil Padrão na hora.
    rig.core.profiles().delete(&id, true).await.unwrap();
    eventually("perfil padrão", || lx(&rig, 0) > 5_000).await;

    stop.store(true, Ordering::Relaxed);
    client.join().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn disconnecting_a_slot_from_the_ui_says_bye_kicked() {
    let rig = rig().await;
    let session = welcomed(&rig, 1).await;
    rig.core.disconnect_slot(SlotId::new(0).unwrap()).unwrap();
    let message = tokio::task::spawn_blocking(move || {
        let mut session = session;
        loop {
            match session.recv(Duration::from_secs(3)).unwrap() {
                ControlMessage::Ping => {}
                other => return other,
            }
        }
    })
    .await
    .unwrap();
    assert!(matches!(message, ControlMessage::Bye(b) if b.reason == ByeReason::Kicked));
    eventually("slot livre", || {
        rig.core.slots().snapshot().free_count() == 4
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn changing_the_profile_of_a_slot_applies_at_once() {
    let rig = rig().await;
    let session = welcomed(&rig, 1).await;
    let stop = Arc::new(AtomicBool::new(false));
    let client = hold(&rig, &session, 0.5, Arc::clone(&stop));
    eventually("perfil padrão", || lx(&rig, 0) > 5_000).await;

    let mirrored = rig
        .core
        .profiles()
        .create(profile_input("Espelhado", |p| {
            p.sticks.left.invert_x = true;
        }))
        .await
        .unwrap();
    let slot = SlotId::new(0).unwrap();
    let applied = rig
        .core
        .set_slot_profile(slot, &mirrored.summary.id)
        .await
        .unwrap();
    assert_eq!(applied, mirrored.summary.id);
    assert_eq!(
        rig.core.slot_profiles()[0].as_deref(),
        Some(mirrored.summary.id.as_str())
    );
    eventually("analógico espelhado", || lx(&rig, 0) < -5_000).await;

    stop.store(true, Ordering::Relaxed);
    client.join().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn swapping_two_slots_moves_the_controller_and_its_profile() {
    let rig = rig().await;
    let session = welcomed(&rig, 1).await;
    let mirrored = rig
        .core
        .profiles()
        .create(profile_input("Espelhado", |p| {
            p.sticks.left.invert_x = true;
        }))
        .await
        .unwrap();
    let (p1, p3) = (SlotId::new(0).unwrap(), SlotId::new(2).unwrap());
    rig.core
        .set_slot_profile(p1, &mirrored.summary.id)
        .await
        .unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let client = hold(&rig, &session, 0.5, Arc::clone(&stop));
    eventually("analógico espelhado no P1", || lx(&rig, 0) < -5_000).await;

    rig.core.swap_slots(p1, p3).unwrap();

    // O controle e o perfil foram para o P3, e o input da mesma sessão continua chegando.
    eventually("input no P3 com o perfil", || lx(&rig, 2) < -5_000).await;
    let profiles = rig.core.slot_profiles();
    assert_eq!(profiles[2].as_deref(), Some(mirrored.summary.id.as_str()));
    assert_ne!(profiles[0].as_deref(), Some(mirrored.summary.id.as_str()));
    let snapshot = rig.core.slots().snapshot();
    assert!(snapshot.slots[0].device.is_none());
    assert!(snapshot.slots[2].device.is_some());

    stop.store(true, Ordering::Relaxed);
    client.join().unwrap();
    // Dois slots vazios: nada a trocar.
    assert!(rig.core.swap_slots(p1, SlotId::new(1).unwrap()).is_err());
}

/// Responde aos `ping` do server com `pong`, como um client de verdade, até `stop`.
fn answer_pings(
    mut session: Box<ControlSession>,
    stop: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            if let Ok(ControlMessage::Ping) = session.recv(Duration::from_millis(100)) {
                let _ = session.send(&ControlMessage::Pong);
            }
        }
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn metrics_report_rtt_rate_and_quality_per_slot() {
    let mut rig = rig().await;
    let session = welcomed(&rig, 1).await;
    let stop = Arc::new(AtomicBool::new(false));
    let sender = hold(&rig, &session, 0.5, Arc::clone(&stop));
    let responder = answer_pings(session, Arc::clone(&stop));

    eventually_async("métricas com RTT", || {
        let metrics = rig.core.metrics();
        async move {
            metrics
                .slots
                .first()
                .is_some_and(|m| m.rtt_last.is_some() && m.rate_hz > 100.0 && m.quality.is_some())
        }
    })
    .await;
    let metrics = rig.core.metrics();
    let slot = &metrics.slots[0];
    assert_eq!(slot.slot, SlotId::new(0).unwrap());
    // Loopback: o RTT do canal de controle fica em poucos ms.
    assert!(
        slot.rtt_last.unwrap() < Duration::from_millis(50),
        "{:?}",
        slot.rtt_last
    );
    assert!(slot.rtt_p[1] >= slot.rtt_p[0]);
    assert!(
        (150.0..=300.0).contains(&slot.rate_hz),
        "taxa {}",
        slot.rate_hz
    );
    assert!(slot.processing_p[1] > Duration::ZERO);
    assert!(matches!(
        slot.quality,
        Some(Quality::Excellent | Quality::Good)
    ));

    // Com a janela à vista, o evento sai a 4 Hz.
    let tick = next_matching(&mut rig.events, |event| match event {
        CoreEvent::Metrics(snapshot) if !snapshot.slots.is_empty() => Some(snapshot.clone()),
        _ => None,
    })
    .await;
    assert_eq!(tick.slots[0].slot, SlotId::new(0).unwrap());

    stop.store(true, Ordering::Relaxed);
    sender.join().unwrap();
    responder.join().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn hidden_window_gets_far_fewer_metric_events() {
    let mut rig = rig().await;
    rig.core.set_ui_visible(false);
    let _ = rig.events.try_recv();
    tokio::time::sleep(Duration::from_millis(1200)).await;
    let mut ticks = 0;
    while let Ok(event) = rig.events.try_recv() {
        if matches!(event, CoreEvent::Metrics(_)) {
            ticks += 1;
        }
    }
    // Visível seriam ~4 por segundo; escondida, um a cada 5 s.
    assert!(ticks <= 1, "{ticks} eventos com a janela escondida");
}

// ------------------------------------------------------------------ confirmação e configurações

#[tokio::test(flavor = "multi_thread")]
async fn new_devices_wait_for_the_users_answer_when_confirmation_is_on() {
    let mut rig = rig().await;
    rig.core
        .settings()
        .modify(|s| s.require_approval = true)
        .await
        .unwrap();
    // A opção liga ao vivo, sem reiniciar.
    next_matching(&mut rig.events, |e| {
        matches!(e, CoreEvent::Settings(_)).then_some(())
    })
    .await;

    // Aceito.
    let addr = rig.control_addr();
    let auth = token(&rig.core);
    let pending = tokio::spawn(connect(addr, Trust::FirstUse, who(1), auth));
    let prompt = next_matching(&mut rig.events, |e| match e {
        CoreEvent::Approval(ApprovalEvent::Requested(p)) => Some(p.clone()),
        _ => None,
    })
    .await;
    assert_eq!(prompt.device_id, who(1).device_id.as_str());
    assert_eq!(prompt.name, "PadGaming Sim");
    assert!(rig.core.answer_approval(prompt.request_id, true));
    assert!(matches!(pending.await.unwrap(), Outcome::Welcome(_)));
    assert!(
        !rig.core.answer_approval(prompt.request_id, true),
        "um pedido só se responde uma vez"
    );

    // Recusado: nada é gravado.
    let auth = token(&rig.core);
    let pending = tokio::spawn(connect(addr, Trust::FirstUse, who(2), auth));
    let prompt = next_matching(&mut rig.events, |e| match e {
        CoreEvent::Approval(ApprovalEvent::Requested(p)) => Some(p.clone()),
        _ => None,
    })
    .await;
    rig.core.answer_approval(prompt.request_id, false);
    assert!(
        matches!(pending.await.unwrap(), Outcome::Rejected(r) if r.reason == RejectReason::BadAuth)
    );
    assert!(
        rig.core
            .devices()
            .find(who(2).device_id.as_str())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unanswered_confirmation_expires_and_closes_the_popup() {
    let mut rig = rig().await;
    rig.core
        .settings()
        .modify(|s| s.require_approval = true)
        .await
        .unwrap();
    let auth = token(&rig.core);
    let pending = tokio::spawn(connect(rig.control_addr(), Trust::FirstUse, who(1), auth));
    let prompt = next_matching(&mut rig.events, |e| match e {
        CoreEvent::Approval(ApprovalEvent::Requested(p)) => Some(p.clone()),
        _ => None,
    })
    .await;
    // Ninguém responde: o servidor desiste em `approval_timeout` (2 s no teste).
    let resolved = next_matching(&mut rig.events, |e| match e {
        CoreEvent::Approval(ApprovalEvent::Resolved {
            request_id,
            accepted,
        }) => Some((*request_id, *accepted)),
        _ => None,
    })
    .await;
    assert_eq!(resolved, (prompt.request_id, false));
    assert!(matches!(pending.await.unwrap(), Outcome::Rejected(_)));
    assert!(!rig.core.answer_approval(prompt.request_id, true));
}

#[tokio::test(flavor = "multi_thread")]
async fn ports_changed_in_the_settings_are_reported_as_pending_a_restart() {
    let rig = rig().await;
    // Com o override do teste as portas em uso (efêmeras) já diferem das configuradas.
    let before = rig.core.ports_pending_restart();
    assert!(before.is_some());
    rig.core
        .settings()
        .modify(|s| {
            s.ports = Ports {
                control: 51000,
                input: 51001,
                discovery: 51002,
            };
        })
        .await
        .unwrap();
    assert_eq!(rig.core.ports_pending_restart().unwrap().control, 51000);
}

// ------------------------------------------------------------------ descoberta, QR e slots

#[tokio::test(flavor = "multi_thread")]
async fn the_broadcast_answer_tracks_the_free_slots() {
    let rig = rig().await;
    let target = rig.core.discovery().unwrap().broadcast_addr().unwrap();
    let free = move || {
        padgaming_sim::discover::by_broadcast(&[target], Duration::from_millis(150))
            .unwrap()
            .first()
            .map(|f| (f.free, f.tcp, f.udp, f.name.clone()))
    };
    let (n, tcp, udp, name) = tokio::task::spawn_blocking(free).await.unwrap().unwrap();
    assert_eq!(n, 4);
    assert_eq!(name, "PC de teste");
    assert_eq!(
        (tcp, udp),
        (rig.core.ports().control, rig.core.ports().input)
    );

    let _session = welcomed(&rig, 1).await;
    eventually_async("free = 3", || async move {
        tokio::task::spawn_blocking(free)
            .await
            .unwrap()
            .is_some_and(|(n, ..)| n == 3)
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_pairing_offer_has_a_qr_a_pin_and_a_uri_that_actually_pairs() {
    let dir = tempfile::tempdir().unwrap();
    let driver = Arc::new(MockDriver::default());
    let mut cfg = config(dir.path(), &driver);
    cfg.bind_ip = "0.0.0.0".parse().unwrap(); // para o IP da LAN da URI ser alcançável
    let core = Core::start(cfg).await.unwrap();
    if core.local_addresses().is_empty() {
        eprintln!("sem interface de LAN: teste pulado");
        core.shutdown().await;
        return;
    }
    let offer = core.start_pairing().unwrap();
    assert!(offer.qr_svg.contains("<svg"));
    assert_eq!(offer.pin.len(), 6);
    assert_eq!(offer.port, core.ports().control);
    assert_eq!(offer.fingerprint_short.len(), 16);

    let uri = PairingUri::parse(&offer.uri).unwrap();
    assert_eq!(uri.fingerprint, core.fingerprint());
    assert_eq!(uri.name, "PC de teste");
    assert_eq!(uri.hosts, offer.hosts);

    // O celular usa o que está no QR: pinning do fingerprint e o token.
    let addr = SocketAddr::new("127.0.0.1".parse().unwrap(), uri.port);
    let auth = Auth::PairingToken {
        token: Secret::new(Bytes(*uri.pairing_token.expose())),
    };
    let outcome = connect(addr, Trust::Pinned(uri.fingerprint), who(1), auth).await;
    assert!(matches!(outcome, Outcome::Welcome(_)));

    // E o PIN também pareia um segundo aparelho.
    let pin = Auth::Pin {
        pin: Secret::new(offer.pin.clone()),
    };
    let outcome = connect(addr, Trust::FirstUse, who(2), pin).await;
    assert!(matches!(outcome, Outcome::Welcome(_)));

    // Cancelar invalida o que estava aberto.
    let again = core.start_pairing().unwrap();
    core.cancel_pairing();
    let stale = Auth::Pin {
        pin: Secret::new(again.pin),
    };
    let outcome = connect(addr, Trust::FirstUse, who(3), stale).await;
    assert!(matches!(outcome, Outcome::Rejected(_)));
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slot_changes_reach_the_ui_and_shutdown_says_bye() {
    let mut rig = rig().await;
    let session = welcomed(&rig, 1).await;
    let snapshot = next_matching(&mut rig.events, |e| match e {
        CoreEvent::Slots(s) if s.free_count() == 3 => Some(Arc::clone(s)),
        _ => None,
    })
    .await;
    assert!(snapshot.slots[0].device.is_some());

    let Rig { core, .. } = rig;
    let reader = tokio::task::spawn_blocking(move || {
        let mut session = session;
        loop {
            match session.recv(Duration::from_secs(5)).unwrap() {
                ControlMessage::Ping => {}
                other => return other,
            }
        }
    });
    core.shutdown().await;
    assert!(matches!(
        reader.await.unwrap(),
        ControlMessage::Bye(b) if b.reason == ByeReason::Shutdown
    ));
}
