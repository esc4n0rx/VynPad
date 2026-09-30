//! O simulador contra o servidor de controle TLS de verdade (loopback + `MockDriver`).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use padgaming_core::{SlotManager, SlotView};
use padgaming_driver::{MockDriver, SlotId};
use padgaming_net::control::{
    ApprovalRequest, ControlConfig, ControlDeps, ControlEvent, ControlServer, DefaultProfiles,
    SessionEnd,
};
use padgaming_net::identity::ServerIdentity;
use padgaming_net::input::{InputConfig, InputServer};
use padgaming_net::pairing::{
    DeviceRecord, DeviceStore, PairingConfig, PairingManager, StoreError,
};
use padgaming_protocol::Secret;
use padgaming_protocol::control::{
    Auth, Bye, ByeReason, Bytes, Config, ControlMessage, RejectReason, Uuid,
};
use padgaming_sim::client::{self, ClientConfig};
use padgaming_sim::control::{self, ControlSession, Identity, Outcome, Trust};
use padgaming_sim::link::FaultConfig;
use padgaming_sim::source::{Pattern, PatternKind};
use tokio::sync::{broadcast, mpsc};

#[derive(Clone, Default)]
struct MemStore(Arc<Mutex<HashMap<String, DeviceRecord>>>);

impl DeviceStore for MemStore {
    fn find(&self, id: &Uuid) -> Result<Option<DeviceRecord>, StoreError> {
        Ok(self.0.lock().unwrap().get(id.as_str()).cloned())
    }
    fn save(&self, record: DeviceRecord) -> Result<(), StoreError> {
        self.0
            .lock()
            .unwrap()
            .insert(record.device_id.as_str().to_owned(), record);
        Ok(())
    }
}

struct Rig {
    control: ControlServer,
    input: Arc<InputServer>,
    driver: Arc<MockDriver>,
    slots: Arc<SlotManager>,
    pairing: Arc<PairingManager<MemStore>>,
    store: MemStore,
    fingerprint: [u8; 32],
    events: broadcast::Receiver<ControlEvent>,
}

fn quick() -> ControlConfig {
    ControlConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        hello_timeout: Duration::from_millis(400),
        ping_after: Duration::from_millis(100),
        dead_after: Duration::from_millis(500),
        approval_timeout: Duration::from_millis(500),
        ..ControlConfig::default()
    }
}

async fn rig(config: ControlConfig) -> Rig {
    let driver = Arc::new(MockDriver::default());
    let slots = Arc::new(SlotManager::new(driver.clone()));
    let input = Arc::new(
        InputServer::start(
            InputConfig {
                bind: "127.0.0.1:0".parse().unwrap(),
                poll_interval: Duration::from_millis(20),
                ..InputConfig::default()
            },
            slots.clone(),
        )
        .unwrap(),
    );
    let identity = ServerIdentity::generate().unwrap();
    let fingerprint = identity.fingerprint();
    let store = MemStore::default();
    let pairing = Arc::new(PairingManager::new(store.clone(), PairingConfig::default()));
    let control = ControlServer::start(
        config,
        ControlDeps {
            identity,
            server_name: "PC de teste".into(),
            pairing: pairing.clone(),
            slots: slots.clone(),
            input: input.clone(),
            profiles: Arc::new(DefaultProfiles::default()),
        },
    )
    .await
    .unwrap();
    let events = control.subscribe();
    Rig {
        control,
        input,
        driver,
        slots,
        pairing,
        store,
        fingerprint,
        events,
    }
}

fn device(n: u8) -> Identity {
    Identity::simulated(Uuid::parse(&format!("00000000-0000-4000-8000-0000000000{n:02}")).unwrap())
}

fn token_auth(rig: &Rig) -> Auth {
    Auth::PairingToken {
        token: Secret::new(Bytes(*rig.pairing.issue_pairing_token().expose())),
    }
}

async fn connect(addr: SocketAddr, trust: Trust, who: Identity, auth: Auth) -> Outcome {
    tokio::task::spawn_blocking(move || control::connect(addr, trust, &who, auth).unwrap())
        .await
        .unwrap()
}

async fn welcomed(
    addr: SocketAddr,
    trust: Trust,
    who: Identity,
    auth: Auth,
) -> Box<ControlSession> {
    match connect(addr, trust, who, auth).await {
        Outcome::Welcome(session) => session,
        Outcome::Rejected(r) => panic!("recusado: {r:?}"),
    }
}

fn rejected(outcome: Outcome) -> padgaming_protocol::control::Reject {
    match outcome {
        Outcome::Rejected(r) => r,
        Outcome::Welcome(w) => panic!("deveria recusar, mas veio {:?}", w.welcome),
    }
}

async fn next_event(rx: &mut broadcast::Receiver<ControlEvent>) -> ControlEvent {
    tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await
        .expect("evento não chegou")
        .unwrap()
}

async fn eventually(what: &str, mut cond: impl FnMut() -> bool) {
    for _ in 0..200 {
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("não aconteceu: {what}");
}

#[tokio::test(flavor = "multi_thread")]
async fn full_handshake_then_input_drives_the_virtual_pad() {
    let mut rig = rig(quick()).await;
    let addr = rig.control.local_addr();
    let session = welcomed(addr, Trust::FirstUse, device(1), token_auth(&rig)).await;

    let w = &session.welcome;
    assert_eq!(w.proto_ver, 1);
    assert_eq!(w.server_name, "PC de teste");
    assert_eq!(w.config.slot, 0);
    assert!(
        w.device_token.is_some(),
        "primeiro pareamento emite o token"
    );
    assert_eq!(session.fingerprint, rig.fingerprint);
    assert_eq!(w.udp_port, rig.input.local_addr().port());

    assert!(matches!(
        next_event(&mut rig.events).await,
        ControlEvent::Paired { .. }
    ));
    assert!(matches!(
        next_event(&mut rig.events).await,
        ControlEvent::SessionStarted { slot, .. } if slot.index() == 0
    ));

    // Input UDP com a chave recebida no welcome.
    let udp = SocketAddr::new(addr.ip(), w.udp_port);
    let config = ClientConfig {
        server: udp,
        session_id: w.session_id,
        key: w.udp_key.expose().0,
        rate_hz: 250,
        fault: FaultConfig::default(),
        seed: 1,
    };
    let report = tokio::task::spawn_blocking(move || {
        let mut source = Pattern::new(PatternKind::Circle, 1);
        client::run(
            &config,
            &mut source,
            &AtomicBool::new(false),
            Some(Duration::from_millis(600)),
            &mut |_| {},
        )
        .unwrap()
    })
    .await
    .unwrap();
    assert!(report.pongs >= 1, "{report:?}");
    let lxs: Vec<i16> = rig.driver.reports().iter().map(|(_, r)| r.lx).collect();
    assert!(lxs.iter().any(|x| *x > 10_000), "o input não chegou ao pad");
    assert_eq!(rig.driver.plugged_slots(), vec![SlotId::new(0).unwrap()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn reconnecting_with_the_device_token_returns_to_the_same_slot() {
    let rig = rig(quick()).await;
    let addr = rig.control.local_addr();
    let first = welcomed(addr, Trust::FirstUse, device(1), token_auth(&rig)).await;
    let token = *first.welcome.device_token.clone().unwrap().expose();
    let fp = first.fingerprint;
    let first_session = first.welcome.session_id;
    let first_key = *first.welcome.udp_key.expose();
    // Outro dispositivo pega o slot 1; o primeiro cai e volta.
    let second = welcomed(addr, Trust::FirstUse, device(2), token_auth(&rig)).await;
    assert_eq!(second.welcome.config.slot, 1);
    first.drop_connection();
    eventually("slot 0 suspenso", || {
        matches!(
            rig.slots.snapshot().slots[0].state,
            SlotView::Suspended { .. }
        )
    })
    .await;
    assert_eq!(rig.control.session_count(), 1);

    let auth = Auth::DeviceToken {
        token: Secret::new(token),
    };
    let again = welcomed(addr, Trust::Pinned(fp), device(1), auth).await;
    assert_eq!(again.welcome.config.slot, 0, "volta ao mesmo slot");
    assert!(again.welcome.device_token.is_none());
    assert_ne!(again.welcome.session_id, first_session, "sessão nova");
    assert_ne!(*again.welcome.udp_key.expose(), first_key, "chave nova");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_revoked_device_gets_reject_revoked() {
    let rig = rig(quick()).await;
    let addr = rig.control.local_addr();
    let first = welcomed(addr, Trust::FirstUse, device(1), token_auth(&rig)).await;
    let token = *first.welcome.device_token.clone().unwrap().expose();
    first.drop_connection();

    let id = device(1).device_id;
    let mut record = rig.store.find(&id).unwrap().unwrap();
    record.revoked = true;
    rig.store.save(record).unwrap();

    let auth = Auth::DeviceToken {
        token: Secret::new(token),
    };
    let r = rejected(connect(addr, Trust::FirstUse, device(1), auth).await);
    assert_eq!(r.reason, RejectReason::Revoked);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_wrong_credential_is_bad_auth() {
    let rig = rig(quick()).await;
    let auth = Auth::PairingToken {
        token: Secret::new(Bytes([7; 16])),
    };
    let r = rejected(connect(rig.control.local_addr(), Trust::FirstUse, device(1), auth).await);
    assert_eq!(r.reason, RejectReason::BadAuth);
    assert_eq!(rig.control.session_count(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_incompatible_version_gets_reject_version_with_the_supported_list() {
    let rig = rig(quick()).await;
    let mut who = device(1);
    who.proto_ver = 2;
    let auth = token_auth(&rig);
    let r = rejected(connect(rig.control.local_addr(), Trust::FirstUse, who, auth).await);
    assert_eq!(r.reason, RejectReason::Version);
    assert_eq!(r.supported_versions, Some(vec![1]));
    assert!(r.message.is_some_and(|m| m.contains('2')));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_fifth_device_gets_no_slot() {
    let rig = rig(quick()).await;
    let addr = rig.control.local_addr();
    let mut keep = Vec::new();
    for n in 1..=4 {
        keep.push(welcomed(addr, Trust::FirstUse, device(n), token_auth(&rig)).await);
    }
    let auth = token_auth(&rig);
    let r = rejected(connect(addr, Trust::FirstUse, device(5), auth).await);
    assert_eq!(r.reason, RejectReason::NoSlot);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pinned_client_refuses_another_certificate() {
    let rig = rig(quick()).await;
    let addr = rig.control.local_addr();
    let auth = token_auth(&rig);
    let result = tokio::task::spawn_blocking(move || {
        control::connect(addr, Trust::Pinned([9; 32]), &device(1), auth)
    })
    .await
    .unwrap();
    assert!(result.is_err());
    assert_eq!(rig.control.session_count(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn approval_is_asked_only_for_new_devices_and_can_deny() {
    let rig = rig(quick()).await;
    let addr = rig.control.local_addr();
    let (tx, mut asks) = mpsc::channel::<ApprovalRequest>(4);
    rig.control.set_approver(Some(tx));
    rig.control.set_require_approval(true);

    // O usuário recusa.
    let auth = token_auth(&rig);
    let denied = tokio::spawn(connect(addr, Trust::FirstUse, device(1), auth));
    let ask = asks.recv().await.unwrap();
    assert_eq!(ask.name, "PadGaming Sim");
    ask.reply.send(false).unwrap();
    assert_eq!(
        rejected(denied.await.unwrap()).reason,
        RejectReason::BadAuth
    );
    assert!(rig.store.find(&device(1).device_id).unwrap().is_none());

    // O usuário aceita.
    let auth = token_auth(&rig);
    let accepted = tokio::spawn(connect(addr, Trust::FirstUse, device(1), auth));
    asks.recv().await.unwrap().reply.send(true).unwrap();
    let session = match accepted.await.unwrap() {
        Outcome::Welcome(s) => s,
        Outcome::Rejected(r) => panic!("{r:?}"),
    };

    // A reconexão com o device_token não pergunta de novo.
    let token = *session.welcome.device_token.clone().unwrap().expose();
    session.drop_connection();
    let auth = Auth::DeviceToken {
        token: Secret::new(token),
    };
    welcomed(addr, Trust::FirstUse, device(1), auth).await;
    assert!(asks.try_recv().is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn approval_without_an_answer_times_out_and_without_a_ui_denies() {
    let rig = rig(quick()).await;
    let addr = rig.control.local_addr();
    rig.control.set_require_approval(true);

    // Sem UI conectada: recusa na hora.
    let auth = token_auth(&rig);
    let r = rejected(connect(addr, Trust::FirstUse, device(1), auth).await);
    assert_eq!(r.reason, RejectReason::BadAuth);

    // Com UI que não responde: recusa quando o prazo acaba.
    let (tx, mut asks) = mpsc::channel::<ApprovalRequest>(4);
    rig.control.set_approver(Some(tx));
    let auth = token_auth(&rig);
    let pending = tokio::spawn(connect(addr, Trust::FirstUse, device(1), auth));
    let held = asks.recv().await.unwrap();
    let r = rejected(pending.await.unwrap());
    assert_eq!(r.reason, RejectReason::BadAuth);
    drop(held);
}

#[tokio::test(flavor = "multi_thread")]
async fn keepalive_pings_are_answered_and_silence_closes_the_session() {
    let mut rig = rig(quick()).await;
    let addr = rig.control.local_addr();
    let session = welcomed(addr, Trust::FirstUse, device(1), token_auth(&rig)).await;
    let (mut session, session_id) = {
        let id = session.welcome.session_id;
        (session, id)
    };

    // O server manda ping depois de `ping_after` sem enviar nada.
    let (session, message) = tokio::task::spawn_blocking(move || {
        let m = session.recv(Duration::from_secs(2)).unwrap();
        (session, m)
    })
    .await
    .unwrap();
    assert_eq!(message, ControlMessage::Ping);
    let mut session = session;

    // Responde e segue vivo por mais que `dead_after`, só trocando ping/pong.
    let mut session = tokio::task::spawn_blocking(move || {
        for _ in 0..8 {
            session.send(&ControlMessage::Pong).unwrap();
            let m = session.recv(Duration::from_secs(2)).unwrap();
            assert_eq!(m, ControlMessage::Ping);
            std::thread::sleep(Duration::from_millis(80));
        }
        session
    })
    .await
    .unwrap();
    assert_eq!(rig.control.session_count(), 1);

    // Agora fica mudo: em `dead_after` o server manda bye{timeout} e fecha.
    let last = tokio::task::spawn_blocking(move || {
        loop {
            match session.recv(Duration::from_secs(3)) {
                Ok(ControlMessage::Ping) => {}
                other => break other.map(|m| (m, session)),
            }
        }
    })
    .await
    .unwrap();
    let (message, _session) = last.unwrap();
    assert!(matches!(
        message,
        ControlMessage::Bye(Bye {
            reason: ByeReason::Timeout,
            ..
        })
    ));
    loop {
        if let ControlEvent::SessionEnded {
            reason,
            session_id: id,
            ..
        } = next_event(&mut rig.events).await
        {
            assert_eq!((id, reason), (session_id, SessionEnd::Timeout));
            break;
        }
    }
    // O slot fica reservado para a reconexão.
    let slot = &rig.slots.snapshot().slots[0];
    assert!(matches!(slot.state, SlotView::Suspended { .. }), "{slot:?}");
    assert_eq!(slot.device.as_ref(), Some(&device(1).device_id));
}

#[tokio::test(flavor = "multi_thread")]
async fn bye_from_the_client_frees_the_slot_and_a_drop_only_suspends_it() {
    let rig = rig(quick()).await;
    let addr = rig.control.local_addr();
    let mut polite = welcomed(addr, Trust::FirstUse, device(1), token_auth(&rig)).await;
    let rude = welcomed(addr, Trust::FirstUse, device(2), token_auth(&rig)).await;
    assert_eq!(rig.slots.snapshot().free_count(), 2);

    let polite = tokio::task::spawn_blocking(move || {
        polite
            .send(&ControlMessage::Bye(Bye {
                reason: ByeReason::User,
                message: None,
            }))
            .unwrap();
        polite
    })
    .await
    .unwrap();
    rude.drop_connection();
    eventually("sessões encerradas", || rig.control.session_count() == 0).await;
    drop(polite);

    // O `bye` liberou o slot 0; a queda deixou o slot 1 reservado.
    assert!(rig.slots.snapshot().slots[0].device.is_none());
    assert!(rig.slots.snapshot().slots[1].device.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn config_pushes_and_kicks_reach_the_client() {
    let rig = rig(quick()).await;
    let addr = rig.control.local_addr();
    let session = welcomed(addr, Trust::FirstUse, device(1), token_auth(&rig)).await;
    let session_id = session.welcome.session_id;
    let slot = SlotId::new(0).unwrap();

    assert!(rig.control.send_config(
        slot,
        Config {
            slot: 3, // ignorado: o server usa o slot real
            send_rate_hz: 120,
            gyro_enabled: true,
            rumble_enabled: false,
            touch_haptics: false,
        }
    ));
    assert!(rig.control.end_session(session_id, ByeReason::Kicked));
    let messages = tokio::task::spawn_blocking(move || {
        let mut session = session;
        let mut seen = Vec::new();
        while let Ok(m) = session.recv(Duration::from_secs(2)) {
            if m != ControlMessage::Ping {
                seen.push(m);
            }
            if matches!(seen.last(), Some(ControlMessage::Bye(_))) {
                break;
            }
        }
        seen
    })
    .await
    .unwrap();
    assert_eq!(messages.len(), 2, "{messages:?}");
    assert!(matches!(
        &messages[0],
        ControlMessage::Config(c) if c.slot == 0 && c.gyro_enabled && c.send_rate_hz == 120
    ));
    assert!(matches!(
        &messages[1],
        ControlMessage::Bye(Bye {
            reason: ByeReason::Kicked,
            ..
        })
    ));
    eventually("slot liberado", || {
        rig.slots.snapshot().slots[0].device.is_none()
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn garbage_before_the_hello_is_bad_request_and_silence_is_closed() {
    use std::io::{Read, Write};
    let rig = rig(quick()).await;
    let addr = rig.control.local_addr();

    // Sem TLS nenhum: o server só fecha, sem sessão.
    let closed = tokio::task::spawn_blocking(move || {
        let mut tcp = std::net::TcpStream::connect(addr).unwrap();
        tcp.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        tcp.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();
        let mut buf = [0u8; 64];
        // Um alerta TLS ou o fechamento; nunca uma sessão.
        matches!(tcp.read(&mut buf), Ok(_) | Err(_))
    })
    .await
    .unwrap();
    assert!(closed);
    assert_eq!(rig.control.session_count(), 0);

    // Conexão que nunca manda o hello é fechada em `hello_timeout` (400 ms).
    let dropped = tokio::task::spawn_blocking(move || {
        let mut tcp = std::net::TcpStream::connect(addr).unwrap();
        tcp.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let start = std::time::Instant::now();
        let mut buf = [0u8; 8];
        let n = tcp.read(&mut buf);
        (matches!(n, Ok(0) | Err(_)), start.elapsed())
    })
    .await
    .unwrap();
    assert!(dropped.0);
    assert!(dropped.1 < Duration::from_secs(2), "{:?}", dropped.1);
}

#[tokio::test(flavor = "multi_thread")]
async fn pending_connections_are_limited() {
    let rig = rig(ControlConfig {
        max_pending: 2,
        hello_timeout: Duration::from_secs(5),
        ..quick()
    })
    .await;
    let addr = rig.control.local_addr();
    let held: Vec<_> = (0..2)
        .map(|_| std::net::TcpStream::connect(addr).unwrap())
        .collect();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // A terceira é fechada sem nem tentar o TLS.
    let third = tokio::task::spawn_blocking(move || {
        use std::io::Read;
        let mut tcp = std::net::TcpStream::connect(addr).unwrap();
        tcp.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut buf = [0u8; 8];
        matches!(tcp.read(&mut buf), Ok(0) | Err(_))
    })
    .await
    .unwrap();
    assert!(third);
    drop(held);
}

#[tokio::test(flavor = "multi_thread")]
async fn shutdown_says_bye_to_everyone() {
    let rig = rig(quick()).await;
    let addr = rig.control.local_addr();
    let session = welcomed(addr, Trust::FirstUse, device(1), token_auth(&rig)).await;
    let Rig { control, input, .. } = rig;
    let reader = tokio::task::spawn_blocking(move || {
        let mut session = session;
        loop {
            match session.recv(Duration::from_secs(3)).unwrap() {
                ControlMessage::Ping => {}
                other => return other,
            }
        }
    });
    control.shutdown().await;
    assert!(matches!(
        reader.await.unwrap(),
        ControlMessage::Bye(Bye {
            reason: ByeReason::Shutdown,
            ..
        })
    ));
    drop(input);
}
