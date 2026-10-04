//! Medições com o **`ViGEmBus` de verdade** (T-702): latência do pacote UDP até o `XInputGetState`
//! e uso de CPU do server. Precisam do driver instalado e de um Windows, então ficam
//! `#[ignore]`; rode com:
//!
//! ```text
//! cargo test -p vynpad-server --release --test xinput_latency -- --ignored --nocapture --test-threads=1
//! ```
#![cfg(windows)]
#![allow(unsafe_code)] // `GetProcessTimes`, só nas medições
#![allow(
    clippy::cast_precision_loss, // contagens e microssegundos pequenos viram f64 só para estatística
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::borrow_as_ptr
)]

use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use vynpad_driver::SlotId;
use vynpad_driver::xinput::XInputProbe;
use vynpad_net::control::ControlConfig;
use vynpad_protocol::control::{Auth, Bytes, Uuid};
use vynpad_protocol::{Buttons, InputState, MAX_PACKET_LEN, Payload, Role, Secret, Session};
use vynpad_server::{Core, CoreConfig};
use vynpad_sim::client::{self, ClientConfig};
use vynpad_sim::control::{self, Identity, Outcome, Trust};
use vynpad_sim::link::FaultConfig;
use vynpad_sim::source::Script;
use vynpad_storage::Ports;

async fn core() -> (Core, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut config = CoreConfig::new(dir.path());
    // Driver real (o padrão no Windows), tudo em loopback e sem mDNS.
    config.bind_ip = "127.0.0.1".parse().unwrap();
    config.ports_override = Some(Ports {
        control: 0,
        input: 0,
        discovery: 0,
    });
    config.mdns = false;
    config.control = ControlConfig {
        dead_after: Duration::from_secs(120),
        ..ControlConfig::default()
    };
    (Core::start(config).await.unwrap(), dir)
}

async fn welcome(core: &Core, n: u8) -> Box<control::ControlSession> {
    let addr = SocketAddr::new("127.0.0.1".parse().unwrap(), core.ports().control);
    let auth = Auth::PairingToken {
        token: Secret::new(Bytes(*core.pairing().issue_pairing_token().expose())),
    };
    let who = Identity::simulated(
        Uuid::parse(&format!("00000000-0000-4000-8000-0000000000{n:02}")).unwrap(),
    );
    let outcome = tokio::task::spawn_blocking(move || {
        control::connect(addr, Trust::FirstUse, &who, auth).unwrap()
    })
    .await
    .unwrap();
    match outcome {
        Outcome::Welcome(session) => session,
        Outcome::Rejected(r) => panic!("recusado: {r:?}"),
    }
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    let rank = ((p / 100.0) * sorted.len() as f64).ceil().max(1.0) as usize;
    sorted[rank.min(sorted.len()) - 1]
}

/// Do envio do pacote ao botão A visível no `XInputGetState`, em loopback (rede = zero).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "precisa do ViGEmBus instalado"]
async fn udp_packet_to_xinput_latency() {
    const ROUNDS: usize = 500;
    let (core, _dir) = core().await;
    let probe = XInputProbe::load().expect("XInput");
    let session = welcome(&core, 1).await;
    let w = &session.welcome;
    let server: SocketAddr = SocketAddr::new("127.0.0.1".parse().unwrap(), w.udp_port);
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.connect(server).unwrap();
    let mut client = Session::new(
        Role::Client,
        w.session_id,
        &Secret::new(w.udp_key.expose().0),
    );
    let mut buf = [0u8; MAX_PACKET_LEN];
    let mut send = |buttons: Buttons| {
        let state = InputState {
            buttons,
            ..InputState::default()
        };
        let n = client.send(&Payload::Input(state), &mut buf).unwrap();
        socket.send(&buf[..n]).unwrap();
    };

    // O controle só é plugado no primeiro input; espera o índice XInput aparecer.
    send(Buttons::empty());
    let slot = SlotId::new(0).unwrap();
    let mut index = None;
    for _ in 0..200 {
        send(Buttons::empty());
        std::thread::sleep(Duration::from_millis(10));
        index = core.slots().snapshot().slots[usize::from(slot.index())].user_index;
        if index.is_some() {
            break;
        }
    }
    let index = index.expect("o controle virtual não ganhou índice XInput");

    let mut samples = Vec::with_capacity(ROUNDS);
    for _ in 0..ROUNDS {
        // Solto e estável.
        send(Buttons::empty());
        while probe.a_pressed(index) != Some(false) {
            std::hint::spin_loop();
        }
        std::thread::sleep(Duration::from_millis(5));
        // Aperta e mede.
        let start = Instant::now();
        send(Buttons::A);
        loop {
            if probe.a_pressed(index) == Some(true) {
                samples.push(u64::try_from(start.elapsed().as_micros()).unwrap());
                break;
            }
            assert!(
                start.elapsed() < Duration::from_millis(100),
                "o botão não chegou"
            );
            std::hint::spin_loop();
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    samples.sort_unstable();
    let mean = samples.iter().sum::<u64>() / samples.len() as u64;
    println!(
        "latência UDP → XInput (loopback, {ROUNDS} rodadas): p50 {} µs  p95 {} µs  p99 {} µs  max {} µs  média {mean} µs",
        percentile(&samples, 50.0),
        percentile(&samples, 95.0),
        percentile(&samples, 99.0),
        samples.last().unwrap()
    );
    assert!(
        percentile(&samples, 95.0) < 5_000,
        "p95 acima de 5 ms mesmo sem rede"
    );
    drop(session);
    core.shutdown().await;
}

/// CPU total do processo (usuário + kernel), em unidades de 100 ns.
fn process_cpu_100ns() -> u64 {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    let zero = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
    // SAFETY: ponteiros para estruturas locais; o handle pseudo do processo atual é sempre válido.
    unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        );
    }
    let ticks = |t: FILETIME| (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
    ticks(kernel) + ticks(user)
}

fn cpu_percent(window: Duration, work: impl FnOnce()) -> f64 {
    let cpus = std::thread::available_parallelism().map_or(1, usize::from) as f64;
    let before = process_cpu_100ns();
    let start = Instant::now();
    work();
    let wall = start.elapsed().as_secs_f64().max(window.as_secs_f64());
    let used = (process_cpu_100ns() - before) as f64 / 10_000_000.0;
    100.0 * used / wall / cpus
}

/// CPU do server parado e com 4 controles a 250 Hz (todos pelo `ViGEmBus` real).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "precisa do ViGEmBus instalado"]
async fn cpu_idle_and_with_four_controllers() {
    const WINDOW: Duration = Duration::from_secs(8);
    let (core, _dir) = core().await;
    let idle = cpu_percent(WINDOW, || std::thread::sleep(WINDOW));

    let mut sessions = Vec::new();
    for n in 1..=4 {
        sessions.push(welcome(&core, n).await);
    }
    let stop = Arc::new(AtomicBool::new(false));
    let threads: Vec<_> = sessions
        .iter()
        .map(|session| {
            let w = &session.welcome;
            let config = ClientConfig {
                server: SocketAddr::new("127.0.0.1".parse().unwrap(), w.udp_port),
                session_id: w.session_id,
                key: w.udp_key.expose().0,
                rate_hz: 250,
                fault: FaultConfig::default(),
                seed: u64::from(w.session_id),
            };
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                let mut script =
                    Script::parse("[[step]]\nduration = \"60s\"\nlx = 0.5\nbuttons = [\"a\"]\n")
                        .unwrap();
                client::run(&config, &mut script, &stop, None, &mut |_| {}).unwrap();
            })
        })
        .collect();
    std::thread::sleep(Duration::from_secs(1));
    let before = core.input().stats().snapshot();
    // O próprio gerador de carga (4 clients simulados) roda neste processo e entra na conta: o
    // número é um teto do que o server sozinho usa.
    let busy = cpu_percent(WINDOW, || std::thread::sleep(WINDOW));
    let after = core.input().stats().snapshot();
    stop.store(true, Ordering::Relaxed);
    for t in threads {
        t.join().unwrap();
    }
    let inputs = after.inputs - before.inputs;
    println!(
        "CPU do processo: parado {idle:.2}%  |  4 controles a 250 Hz {busy:.2}% (inclui os 4 clients simulados; {inputs} inputs em {WINDOW:?}, p99 de processamento {:?})",
        core.input().stats().processing.percentile(99.0)
    );
    assert!(idle < 1.0, "ocioso {idle:.2}% (meta < 1%)");
    drop(sessions);
    core.shutdown().await;
}
