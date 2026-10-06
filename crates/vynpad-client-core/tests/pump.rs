//! O motor de envio contra o server de input de verdade (loopback + driver de mentira): o estado
//! do controle chega ao pad virtual, o RTT é medido e a vibração volta.

mod common;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use common::{Rig, connect, pinned, rig, token_credential};
use vynpad_client_core::{InputPump, PumpListener, WelcomeInfo};
use vynpad_driver::SlotId;

#[derive(Default)]
struct Spy {
    rtts: Mutex<Vec<u32>>,
    rumbles: Mutex<Vec<(u16, u8, u8)>>,
    stopped: Mutex<Vec<String>>,
}

impl PumpListener for Spy {
    fn on_rtt(&self, rtt_us: u32) {
        self.rtts.lock().unwrap().push(rtt_us);
    }
    fn on_rumble(&self, rumble_id: u16, large: u8, small: u8) {
        self.rumbles.lock().unwrap().push((rumble_id, large, small));
    }
    fn on_stopped(&self, reason: String) {
        self.stopped.lock().unwrap().push(reason);
    }
}

struct Session {
    // A conexão de controle precisa continuar viva: sem ela o server encerra a sessão.
    _control: Arc<vynpad_client_core::ControlConnection>,
    pump: Arc<InputPump>,
    spy: Arc<Spy>,
    sid: u32,
}

async fn session(rig: &Rig) -> Session {
    let control = connect(rig, pinned(rig), token_credential(rig))
        .await
        .unwrap();
    let welcome: WelcomeInfo = control.welcome();
    let session_id = welcome.session_id;
    control.start(Arc::new(NoControlEvents));
    let spy = Arc::new(Spy::default());
    let pump = InputPump::start("127.0.0.1", welcome, spy.clone()).unwrap();
    Session {
        _control: control,
        pump,
        spy,
        sid: session_id,
    }
}

struct NoControlEvents;

impl vynpad_client_core::SessionListener for NoControlEvents {
    fn on_config(&self, _: vynpad_client_core::ConfigInfo) {}
    fn on_closed(&self, _: String) {}
}

/// Espera até `limit` por uma condição; devolve quanto demorou.
async fn eventually(limit: Duration, mut cond: impl FnMut() -> bool) -> Option<Duration> {
    let start = Instant::now();
    while start.elapsed() < limit {
        if cond() {
            return Some(start.elapsed());
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    None
}

fn slot0() -> SlotId {
    SlotId::new(0).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_input_reaches_the_virtual_pad_as_soon_as_it_changes() {
    let rig = rig().await;
    let s = session(&rig).await;

    // A (0x1000) + gatilho direito + analógico esquerdo todo à direita e para cima.
    s.pump.set_input(0x1000, 0, 200, 32767, 32767, 0, -32768);
    let took = eventually(Duration::from_secs(2), || {
        rig.driver.last_report(slot0()).is_some_and(|r| {
            r.buttons == 0x1000 && r.rt == 200 && r.lx == 32767 && r.ly == 32767 && r.ry == -32768
        })
    })
    .await
    .expect("o input não chegou ao pad virtual");
    // Em loopback, do `set_input` ao pad virtual deve levar bem pouco: o teste mede com folga.
    assert!(took < Duration::from_millis(250), "demorou {took:?}");

    // Soltar tudo também chega.
    s.pump.set_input(0, 0, 0, 0, 0, 0, 0);
    eventually(Duration::from_secs(2), || {
        rig.driver
            .last_report(slot0())
            .is_some_and(|r| r.buttons == 0 && r.rt == 0 && r.lx == 0)
    })
    .await
    .expect("o estado neutro não chegou");
    s.pump.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_state_is_resent_as_a_keepalive_even_without_changes() {
    let rig = rig().await;
    let s = session(&rig).await;
    s.pump.set_input(0x2000, 0, 0, 0, 0, 0, 0);
    eventually(Duration::from_secs(2), || {
        rig.driver
            .last_report(slot0())
            .is_some_and(|r| r.buttons == 0x2000)
    })
    .await
    .unwrap();

    let before = rig.driver.reports().len();
    tokio::time::sleep(Duration::from_millis(700)).await;
    let after = rig.driver.reports().len();
    // 10 Hz de keepalive durante 0,7 s: pelo menos alguns pacotes, mas longe do teto de 250 Hz.
    assert!(
        after - before >= 3,
        "poucos pacotes de keepalive: {}",
        after - before
    );
    assert!(
        after - before < 40,
        "muitos pacotes parados: {}",
        after - before
    );
    s.pump.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn rapid_changes_are_capped_at_the_servers_rate() {
    let rig = rig().await;
    let s = session(&rig).await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    let before = rig.driver.reports().len();
    // Milhares de mudanças em 1 s (laço apertado: o sleep do Windows é grosso demais para isto),
    // muito acima do teto de 250 Hz.
    let pump = s.pump.clone();
    let changes = tokio::task::spawn_blocking(move || {
        let start = Instant::now();
        let mut i: i16 = 0;
        let mut n = 0u32;
        while start.elapsed() < Duration::from_secs(1) {
            i = i.wrapping_add(7);
            pump.set_input(0, 0, 0, i, 0, 0, 0);
            n += 1;
        }
        n
    })
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let sent = rig.driver.reports().len() - before;
    assert!(changes > 2_000, "poucas mudanças geradas: {changes}");
    assert!(
        sent <= 300,
        "passou do teto de taxa: {sent} pacotes em ~1 s"
    );
    assert!(sent >= 150, "enviou pouco: {sent}");
    s.pump.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_rtt_is_measured_from_the_pongs() {
    let rig = rig().await;
    let s = session(&rig).await;
    eventually(Duration::from_secs(3), || {
        !s.spy.rtts.lock().unwrap().is_empty()
    })
    .await
    .expect("nenhum pong chegou");
    let rtt = s.spy.rtts.lock().unwrap()[0];
    // Em loopback o RTT é pequeno; a folga cobre uma máquina carregada.
    assert!(rtt < 100_000, "RTT absurdo: {rtt} µs");
    s.pump.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_rumble_comes_back_and_is_acknowledged() {
    let rig = rig().await;
    let s = session(&rig).await;
    // O server só manda UDP depois do primeiro pacote válido: espera o pad existir.
    eventually(Duration::from_secs(2), || {
        rig.driver.last_report(slot0()).is_some()
    })
    .await
    .unwrap();

    // No app do PC o driver virtual chama isto quando o jogo pede vibração.
    rig.input.set_rumble(s.sid, 200, 50);
    eventually(Duration::from_secs(3), || {
        !s.spy.rumbles.lock().unwrap().is_empty()
    })
    .await
    .expect("a vibração não chegou");
    let (id, large, small) = s.spy.rumbles.lock().unwrap()[0];
    assert_ne!(id, 0);
    assert_eq!((large, small), (200, 50));
    s.pump.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn stop_ends_the_engine_once_and_reports_local() {
    let rig = rig().await;
    let s = session(&rig).await;
    s.pump.stop();
    s.pump.stop();
    assert_eq!(*s.spy.stopped.lock().unwrap(), vec!["local".to_owned()]);

    // Depois de parar, um botão novo nunca chega ao pad (o server solta o controle sozinho
    // quando o client se cala, e isso é bom: o relatório neutro é esperado).
    s.pump.set_input(0x1000, 0, 0, 0, 0, 0, 0);
    tokio::time::sleep(Duration::from_millis(400)).await;
    let last = rig.driver.last_report(slot0());
    assert!(
        last.is_none_or(|r| r.buttons == 0),
        "input chegou depois do stop: {last:?}"
    );
}
