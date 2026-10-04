//! O loop de input de ponta a ponta, com sockets UDP reais em loopback e o `MockDriver`.

use std::net::UdpSocket;
use std::sync::Arc;
use std::time::{Duration, Instant};

use padgaming_core::{SlotManager, SlotView};
use padgaming_driver::{MockDriver, SlotId, XReport, xbuttons};
use padgaming_mapping::{CompiledProfile, ProfileCell};
use padgaming_net::input::{InputConfig, InputServer, SessionSpec};
use padgaming_protocol::control::Uuid;
use padgaming_protocol::{
    Buttons, InputState, KEY_LEN, MAX_PACKET_LEN, Motion, Payload, Ping, Role, Secret, Session,
};

const KEY: [u8; KEY_LEN] = [0x5A; KEY_LEN];

struct Rig {
    server: InputServer,
    driver: Arc<MockDriver>,
    slots: Arc<SlotManager>,
    profile: Arc<ProfileCell>,
}

fn rig() -> Rig {
    let driver = Arc::new(MockDriver::default());
    let slots = Arc::new(SlotManager::new(driver.clone()));
    let config = InputConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        poll_interval: Duration::from_millis(20),
        ..InputConfig::default()
    };
    let server = InputServer::start(config, slots.clone()).unwrap();
    Rig {
        server,
        driver,
        slots,
        profile: Arc::new(ProfileCell::new(CompiledProfile::identity())),
    }
}

fn device(n: u8) -> Uuid {
    Uuid::parse(&format!("123e4567-e89b-42d3-a456-4266141740{n:02}")).unwrap()
}

impl Rig {
    /// Registra o dispositivo `n` (slot + sessão UDP) e devolve o client já configurado.
    fn connect(&self, n: u8) -> Client {
        let session_id = 100 + u32::from(n);
        self.slots.assign(&device(n), session_id, None).unwrap();
        self.server.add_session(SessionSpec {
            session_id,
            key: Secret::new(KEY),
            profile: self.profile.clone(),
        });
        Client::new(session_id, self.server.local_addr())
    }
}

struct Client {
    socket: UdpSocket,
    session: Session,
    session_id: u32,
    server: std::net::SocketAddr,
    buf: [u8; MAX_PACKET_LEN],
}

impl Client {
    fn new(session_id: u32, server: std::net::SocketAddr) -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        Self {
            socket,
            session: Session::new(Role::Client, session_id, &Secret::new(KEY)),
            session_id,
            server,
            buf: [0; MAX_PACKET_LEN],
        }
    }

    fn sealed(&mut self, payload: &Payload) -> Vec<u8> {
        let n = self.session.send(payload, &mut self.buf).unwrap();
        self.buf[..n].to_vec()
    }

    fn send(&mut self, payload: &Payload) {
        let packet = self.sealed(payload);
        self.socket.send_to(&packet, self.server).unwrap();
    }

    fn send_input(&mut self, state: InputState) {
        self.send(&Payload::Input(state));
    }

    fn recv_pong(&mut self) -> padgaming_protocol::Pong {
        let mut buf = [0u8; 256];
        let (n, _) = self.socket.recv_from(&mut buf).expect("nenhum Pong chegou");
        match self.session.receive(&buf[..n]).unwrap().payload {
            Payload::Pong(pong) => pong,
            other => panic!("esperava Pong, veio {other:?}"),
        }
    }
}

fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "tempo esgotado esperando: {what}"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn pressed(buttons: Buttons, lx: i16) -> InputState {
    InputState {
        buttons,
        lx,
        lt: 33,
        ..InputState::default()
    }
}

fn slot(i: u8) -> SlotId {
    SlotId::new(i).unwrap()
}

// ------------------------------------------------------------------------------------------

#[test]
fn input_reaches_the_driver_through_the_mapping_pipeline() {
    let rig = rig();
    let mut client = rig.connect(1);
    client.send_input(pressed(Buttons::A | Buttons::START, -1234));

    wait_until("report no driver", || {
        rig.driver.last_report(slot(0)).is_some()
    });
    let report = rig.driver.last_report(slot(0)).unwrap();
    assert_eq!(
        report,
        XReport {
            buttons: xbuttons::A | xbuttons::START,
            lt: 33,
            lx: -1234,
            ..XReport::default()
        }
    );
    let snap = rig.server.stats().snapshot();
    assert_eq!((snap.inputs, snap.dropped()), (1, 0));
}

#[test]
fn a_profile_swap_is_applied_to_the_next_packet() {
    let rig = rig();
    let mut client = rig.connect(1);
    client.send_input(pressed(Buttons::empty(), 1000));
    wait_until("1º report", || rig.driver.last_report(slot(0)).is_some());
    assert_eq!(rig.driver.last_report(slot(0)).unwrap().lx, 1000);

    let mut inverted = padgaming_mapping::Profile::identity();
    inverted.sticks.left.invert_x = true;
    rig.profile
        .store(Arc::new(CompiledProfile::compile(&inverted).unwrap()));
    client.send_input(pressed(Buttons::empty(), 1000));
    wait_until("report invertido", || {
        rig.driver.last_report(slot(0)).unwrap().lx == -1000
    });
}

#[test]
fn motion_packets_feed_the_gyro() {
    let rig = rig();
    let mut profile = padgaming_mapping::Profile::identity();
    profile.gyro.enabled = true;
    profile.gyro.smoothing.enabled = false;
    profile.gyro.deadzone = 0.0;
    rig.profile
        .store(Arc::new(CompiledProfile::compile(&profile).unwrap()));

    let mut client = rig.connect(1);
    // gy = -1000 => -1 rad/s => stick direito para a direita.
    client.send(&Payload::InputMotion(
        InputState::default(),
        Motion {
            gy: -1000,
            ..Motion::default()
        },
    ));
    wait_until("gyro no stick", || {
        rig.driver
            .last_report(slot(0))
            .is_some_and(|r| r.rx > 10_000)
    });
}

#[test]
fn ping_gets_an_immediate_pong_with_the_server_timings() {
    let rig = rig();
    let mut client = rig.connect(1);
    client.send(&Payload::Ping(Ping {
        t_client_us: 123_456_789,
    }));
    let pong = client.recv_pong();
    assert_eq!(pong.t_client_us, 123_456_789);
    assert!(pong.t_server_rx_us > 0);
    assert!(pong.server_proc_us < 5_000, "{}", pong.server_proc_us);
    assert_eq!(rig.server.stats().snapshot().pings, 1);
}

#[test]
fn tampered_replayed_unknown_and_garbage_packets_are_dropped_and_counted() {
    let rig = rig();
    let mut client = rig.connect(1);

    // Um pacote bom.
    client.send_input(pressed(Buttons::A, 1));
    wait_until("pacote bom", || rig.server.stats().snapshot().inputs == 1);

    // Repetido (mesmo seq).
    let packet = client.sealed(&Payload::Input(pressed(Buttons::B, 2)));
    client.socket.send_to(&packet, client.server).unwrap();
    wait_until("2º bom", || rig.server.stats().snapshot().inputs == 2);
    client.socket.send_to(&packet, client.server).unwrap();

    // Adulterado: um bit do payload muda, a tag deixa de bater.
    let mut tampered = client.sealed(&Payload::Input(pressed(Buttons::X, 3)));
    tampered[14] ^= 0x01;
    client.socket.send_to(&tampered, client.server).unwrap();

    // Sessão desconhecida.
    let mut stranger = Client::new(9999, client.server);
    stranger.send_input(pressed(Buttons::Y, 4));

    // Lixo: curto demais, e com formato errado.
    client.socket.send_to(&[1, 2, 3], client.server).unwrap();
    client.socket.send_to(&[0u8; 80], client.server).unwrap();

    wait_until("descartes contados", || {
        rig.server.stats().snapshot().dropped() >= 5
    });
    let snap = rig.server.stats().snapshot();
    assert_eq!(snap.dropped_replay, 1);
    assert_eq!(snap.dropped_auth, 1);
    assert_eq!(snap.dropped_unknown_session, 1);
    assert_eq!(snap.dropped_malformed, 2);
    assert_eq!(snap.inputs, 2, "nada disso pode ter chegado ao driver");
    assert_eq!(
        rig.driver.last_report(slot(0)).unwrap().buttons,
        xbuttons::B
    );
}

#[test]
fn a_dropped_packet_never_changes_the_pad() {
    let rig = rig();
    let mut client = rig.connect(1);
    client.send_input(pressed(Buttons::A, 100));
    wait_until("1º", || rig.server.stats().snapshot().inputs == 1);
    let before = rig.driver.reports().len();

    let mut forged = client.sealed(&Payload::Input(pressed(Buttons::Y, 30_000)));
    forged[20] ^= 0xFF;
    client.socket.send_to(&forged, client.server).unwrap();
    wait_until("descartado", || {
        rig.server.stats().snapshot().dropped_auth == 1
    });
    assert_eq!(rig.driver.reports().len(), before);
}

#[test]
fn roaming_updates_the_peer_and_the_pong_follows_it() {
    let rig = rig();
    let session_id = 101;
    rig.slots.assign(&device(1), session_id, None).unwrap();
    let activity = rig.server.add_session(SessionSpec {
        session_id,
        key: Secret::new(KEY),
        profile: rig.profile.clone(),
    });
    assert_eq!(activity.peer(), None);

    let mut first = Client::new(session_id, rig.server.local_addr());
    first.send_input(pressed(Buttons::empty(), 1));
    wait_until("1º endereço", || {
        activity.peer() == Some(first.socket.local_addr().unwrap())
    });

    // O mesmo aparelho reaparece em outra porta (mesma sessão e mesma sequência de seq).
    let second_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    second_socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let ping = first.sealed(&Payload::Ping(Ping { t_client_us: 7 }));
    second_socket.send_to(&ping, first.server).unwrap();
    wait_until("2º endereço", || {
        activity.peer() == Some(second_socket.local_addr().unwrap())
    });

    let mut buf = [0u8; 256];
    let (n, _) = second_socket
        .recv_from(&mut buf)
        .expect("Pong no novo endereço");
    assert!(matches!(
        first.session.receive(&buf[..n]).unwrap().payload,
        Payload::Pong(_)
    ));
    assert!(activity.idle_for(Instant::now()) < Duration::from_secs(1));
}

#[test]
fn removed_sessions_stop_being_served() {
    let rig = rig();
    let mut client = rig.connect(1);
    client.send_input(pressed(Buttons::A, 1));
    wait_until("ok", || rig.server.stats().snapshot().inputs == 1);

    rig.server.remove_session(client.session_id);
    // O comando é adotado antes do próximo pacote processado.
    client.send_input(pressed(Buttons::B, 2));
    wait_until("descarte", || {
        rig.server.stats().snapshot().dropped_unknown_session == 1
    });
}

#[test]
fn each_session_drives_its_own_slot() {
    let rig = rig();
    let mut a = rig.connect(1);
    let mut b = rig.connect(2);
    a.send_input(pressed(Buttons::A, 1));
    b.send_input(pressed(Buttons::B, 2));
    wait_until("os dois slots", || {
        rig.driver.last_report(slot(0)).is_some() && rig.driver.last_report(slot(1)).is_some()
    });
    assert_eq!(
        rig.driver.last_report(slot(0)).unwrap().buttons,
        xbuttons::A
    );
    assert_eq!(
        rig.driver.last_report(slot(1)).unwrap().buttons,
        xbuttons::B
    );
    assert_eq!(rig.slots.snapshot().slots[1].state, SlotView::Active);
}

#[test]
fn shutdown_is_quick_and_releases_the_port() {
    let rig = rig();
    let addr = rig.server.local_addr();
    let started = Instant::now();
    rig.server.shutdown();
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "{:?}",
        started.elapsed()
    );
    // A porta volta a ficar livre.
    UdpSocket::bind(addr).expect("a porta deveria ter sido liberada");
}

#[cfg(debug_assertions)]
#[test]
fn the_insecure_dev_session_works_with_the_public_key() {
    use padgaming_net::input::{DEV_KEY, DEV_SESSION_ID};

    let rig = rig();
    rig.slots.assign(&device(1), DEV_SESSION_ID, None).unwrap();
    rig.server.add_insecure_dev_session(rig.profile.clone());

    let mut session = Session::new(Role::Client, DEV_SESSION_ID, &Secret::new(DEV_KEY));
    let mut buf = [0u8; MAX_PACKET_LEN];
    let n = session
        .send(&Payload::Input(pressed(Buttons::A, 5)), &mut buf)
        .unwrap();
    UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .send_to(&buf[..n], rig.server.local_addr())
        .unwrap();
    wait_until("dev session", || rig.driver.last_report(slot(0)).is_some());
}

/// Critério da T-301: com 4 simuladores a 250 Hz, o p99 do tempo de processamento fica
/// abaixo de 1 ms. Loopback real, então também confere que nada se perde.
#[test]
fn four_simulators_at_250_hz_keep_p99_under_one_millisecond() {
    const SECONDS: u64 = 3;
    const RATE_HZ: u64 = 250;
    let rig = rig();
    let clients: Vec<Client> = (1..=4).map(|n| rig.connect(n)).collect();

    let threads: Vec<_> = clients
        .into_iter()
        .map(|mut client| {
            std::thread::spawn(move || {
                let period = Duration::from_micros(1_000_000 / RATE_HZ);
                let start = Instant::now();
                for i in 0..SECONDS * RATE_HZ {
                    // Espera ativa: o timer do Windows (~15 ms) não serve para 4 ms.
                    let due = start + period * u32::try_from(i).unwrap();
                    while Instant::now() < due {
                        std::hint::spin_loop();
                    }
                    let lx = i16::try_from(i % 20_000).unwrap();
                    client.send_input(pressed(Buttons::A, lx));
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }

    let expected = 4 * SECONDS * RATE_HZ;
    wait_until("todos os pacotes", || {
        rig.server.stats().snapshot().inputs >= expected * 99 / 100
    });
    let stats = rig.server.stats();
    let snap = stats.snapshot();
    let p99 = stats.processing.percentile(99.0);
    println!(
        "recebidos {} inputs {} descartes {} | p50 {:?} p99 {:?} max {:?}",
        snap.received,
        snap.inputs,
        snap.dropped(),
        stats.processing.percentile(50.0),
        p99,
        stats.processing.max()
    );
    assert_eq!(snap.dropped(), 0);
    assert_eq!(snap.submit_errors, 0);
    assert!(p99 < Duration::from_millis(1), "p99 = {p99:?}");
}

#[test]
fn session_metrics_count_loss_out_of_order_and_battery() {
    let rig = rig();
    let session_id = 101;
    rig.slots.assign(&device(1), session_id, None).unwrap();
    let activity = rig.server.add_session(SessionSpec {
        session_id,
        key: Secret::new(KEY),
        profile: rig.profile.clone(),
    });
    let mut client = Client::new(session_id, rig.server.local_addr());
    let with_battery = |battery| InputState {
        battery,
        ..pressed(Buttons::A, 10)
    };

    client.send_input(with_battery(80));
    let first = client.sealed(&Payload::Input(with_battery(80)));
    client.socket.send_to(&first, client.server).unwrap();
    // Dois pacotes selados e nunca enviados: o servidor os vê como perda.
    let _ = client.sealed(&Payload::Input(with_battery(80)));
    let _ = client.sealed(&Payload::Input(with_battery(80)));
    client.send_input(with_battery(79));
    // O mesmo datagrama de novo: chega repetido (fora de ordem).
    client.socket.send_to(&first, client.server).unwrap();

    wait_until("métricas", || {
        let s = activity.metrics.snapshot();
        s.received == 3 && s.out_of_order == 1
    });
    let s = activity.metrics.snapshot();
    assert_eq!(s.lost, 2);
    assert_eq!(s.battery, Some(79));
    assert!(s.processing_p[1] > Duration::ZERO);
    assert_eq!(s.rtt_last, None, "o RTT vem do canal de controle");
}
