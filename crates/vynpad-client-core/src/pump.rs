//! O motor de envio do input (T-906): leva o estado do controle ao PC por UDP o mais rápido
//! possível, mede o RTT e entrega a vibração que o jogo manda de volta.
//!
//! Duas threads com a mesma sessão cifrada:
//! - **envio**: acorda na hora em que o estado muda (sem esperar um relógio), respeita o teto de
//!   taxa do server (até 250 Hz por padrão), repete o estado a 10 Hz como keepalive e manda um
//!   `ping` a cada 250 ms;
//! - **recepção**: abre os pacotes do server, calcula o RTT dos `pong` e repassa o `rumble`.
//!
//! Nada aloca por pacote: o selo é feito num buffer de pilha.

use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use vynpad_protocol::packet::InputState;
use vynpad_protocol::{
    Buttons, KEY_LEN, MAX_PACKET_LEN, Motion, Payload, Ping, ProtoError, Role, Secret, Session,
};

use crate::{CoreError, MotionSample, WelcomeInfo};

/// Repete o estado nesta cadência mesmo sem mudança (o server usa isso para saber que há vida).
const KEEPALIVE: Duration = Duration::from_millis(100);
/// Cadência dos `ping` (PROTOCOLO §9).
const PING_EVERY: Duration = Duration::from_millis(250);
/// Quanto a recepção espera antes de olhar o cancelamento.
const RECV_POLL: Duration = Duration::from_millis(100);
/// Taxa máxima quando o server não diz nada sensato.
const DEFAULT_RATE_HZ: u16 = 250;

/// Quem acompanha o motor de envio. Implementado pelo Kotlin/Swift.
///
/// Os métodos rodam nas threads do motor: não bloqueie neles.
#[uniffi::export(with_foreign)]
pub trait PumpListener: Send + Sync {
    /// RTT de rede do último `pong`, em µs (já sem o tempo de processamento do server).
    fn on_rtt(&self, rtt_us: u32);

    /// O jogo mandou vibrar. `rumble_id` nunca é 0; os motores vão de 0 a 255.
    fn on_rumble(&self, rumble_id: u16, large: u8, small: u8);

    /// O motor parou. `reason`: `local` (o app pediu), `seq_exhausted` (acabou o contador de
    /// pacotes da sessão: reconectar) ou `socket` (o socket UDP falhou). Chamado uma única vez.
    fn on_stopped(&self, reason: String);
}

/// O estado a enviar. Escrito pela UI a cada toque, lido pela thread de envio.
struct Pending {
    buttons: u16,
    lt: u8,
    rt: u8,
    lx: i16,
    ly: i16,
    rx: i16,
    ry: i16,
    battery: u8,
    rumble_ack_id: u16,
    motion: Option<Motion>,
    /// Há algo novo para mandar já (e não só o keepalive).
    dirty: bool,
}

struct Shared {
    pending: Mutex<Pending>,
    wake: Condvar,
    stop: AtomicBool,
    stopped: AtomicBool,
    session: Mutex<Session>,
    started: Instant,
}

impl Shared {
    fn pending(&self) -> MutexGuard<'_, Pending> {
        self.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn session(&self) -> MutexGuard<'_, Session> {
        self.session.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Relógio monotônico do client, em µs, desde o início do motor.
    fn now_us(&self) -> i64 {
        i64::try_from(self.started.elapsed().as_micros()).unwrap_or(i64::MAX)
    }

    /// Avisa o fim uma única vez, não importa qual thread chegue primeiro.
    fn finish(&self, listener: &dyn PumpListener, reason: &str) {
        self.stop.store(true, Ordering::SeqCst);
        self.wake.notify_all();
        if !self.stopped.swap(true, Ordering::SeqCst) {
            listener.on_stopped(reason.to_owned());
        }
    }
}

/// Envia o input do controle ao PC e recebe pong e rumble.
#[derive(uniffi::Object)]
pub struct InputPump {
    shared: Arc<Shared>,
    listener: Arc<dyn PumpListener>,
    threads: Mutex<Vec<JoinHandle<()>>>,
}

#[uniffi::export]
impl InputPump {
    /// Abre o socket UDP para `host:udp_port` e começa a enviar o estado (inicialmente neutro).
    // O UniFFI entrega o `WelcomeInfo` e o listener por valor.
    #[allow(clippy::needless_pass_by_value)]
    #[uniffi::constructor]
    pub fn start(
        host: &str,
        welcome: WelcomeInfo,
        listener: Arc<dyn PumpListener>,
    ) -> Result<Arc<Self>, CoreError> {
        let ip: IpAddr = host.parse().map_err(|_| CoreError::InvalidArgument {
            reason: format!("endereço inválido: {host}"),
        })?;
        let key: [u8; KEY_LEN] = welcome.udp_key.clone().try_into().map_err(|k: Vec<u8>| {
            CoreError::InvalidArgument {
                reason: format!("a chave UDP deve ter {KEY_LEN} bytes, recebeu {}", k.len()),
            }
        })?;
        let rate = if (10..=500).contains(&welcome.send_rate_hz) {
            welcome.send_rate_hz
        } else {
            DEFAULT_RATE_HZ
        };
        let min_gap = Duration::from_micros(1_000_000 / u64::from(rate));

        let bind: SocketAddr = if ip.is_ipv4() {
            "0.0.0.0:0".parse().map_err(network)?
        } else {
            "[::]:0".parse().map_err(network)?
        };
        let socket = UdpSocket::bind(bind).map_err(network)?;
        socket
            .connect(SocketAddr::new(ip, welcome.udp_port))
            .map_err(network)?;
        socket.set_read_timeout(Some(RECV_POLL)).map_err(network)?;
        let receiver_socket = socket.try_clone().map_err(network)?;

        let shared = Arc::new(Shared {
            pending: Mutex::new(Pending {
                buttons: 0,
                lt: 0,
                rt: 0,
                lx: 0,
                ly: 0,
                rx: 0,
                ry: 0,
                battery: InputState::BATTERY_UNKNOWN,
                rumble_ack_id: 0,
                motion: None,
                dirty: true,
            }),
            wake: Condvar::new(),
            stop: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            session: Mutex::new(Session::new(
                Role::Client,
                welcome.session_id,
                &Secret::new(key),
            )),
            started: Instant::now(),
        });

        let sender = std::thread::Builder::new()
            .name("vynpad-send".to_owned())
            .spawn({
                let shared = Arc::clone(&shared);
                let listener = Arc::clone(&listener);
                move || send_loop(&shared, &socket, min_gap, listener.as_ref())
            })
            .map_err(network)?;
        let receiver = std::thread::Builder::new()
            .name("vynpad-recv".to_owned())
            .spawn({
                let shared = Arc::clone(&shared);
                let listener = Arc::clone(&listener);
                move || receive_loop(&shared, &receiver_socket, listener.as_ref())
            })
            .map_err(network)?;

        Ok(Arc::new(Self {
            shared,
            listener,
            threads: Mutex::new(vec![sender, receiver]),
        }))
    }

    /// Atualiza o estado do controle. Chamado a cada toque: só grava e acorda o envio.
    #[allow(clippy::too_many_arguments)] // Cada eixo é um argumento: evita alocar uma estrutura.
    pub fn set_input(&self, buttons: u16, lt: u8, rt: u8, lx: i16, ly: i16, rx: i16, ry: i16) {
        let mut p = self.shared.pending();
        let next = (buttons & Buttons::all().bits(), lt, rt, lx, ly, rx, ry);
        if next != (p.buttons, p.lt, p.rt, p.lx, p.ly, p.rx, p.ry) {
            (p.buttons, p.lt, p.rt, p.lx, p.ly, p.rx, p.ry) = next;
            p.dirty = true;
            drop(p);
            self.shared.wake.notify_one();
        }
    }

    /// Bateria do aparelho em % (0–100); 255 = desconhecida. Vai junto do próximo pacote.
    pub fn set_battery(&self, percent: u8) {
        self.shared.pending().battery = percent;
    }

    /// Passa a enviar também os sensores de movimento (pacote `0x02`).
    pub fn set_motion(&self, sample: MotionSample) {
        let mut p = self.shared.pending();
        p.motion = Some(sample.into());
        p.dirty = true;
        drop(p);
        self.shared.wake.notify_one();
    }

    /// Volta a enviar só o estado do controle (pacote `0x01`).
    pub fn clear_motion(&self) {
        self.shared.pending().motion = None;
    }

    /// Para o motor e espera as threads terminarem.
    pub fn stop(&self) {
        self.shared.finish(self.listener.as_ref(), "local");
        let threads =
            std::mem::take(&mut *self.threads.lock().unwrap_or_else(PoisonError::into_inner));
        for thread in threads {
            let _ = thread.join();
        }
    }
}

impl Drop for InputPump {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        self.shared.wake.notify_all();
    }
}

fn network(e: impl std::fmt::Display) -> CoreError {
    CoreError::Network {
        detail: e.to_string(),
    }
}

/// Monta o payload a partir do estado pendente.
fn payload(p: &Pending, now_us: i64) -> Payload {
    let state = InputState {
        t_client_us: now_us,
        buttons: Buttons::from_wire(p.buttons),
        lt: p.lt,
        rt: p.rt,
        lx: p.lx,
        ly: p.ly,
        rx: p.rx,
        ry: p.ry,
        rumble_ack_id: p.rumble_ack_id,
        battery: p.battery,
    };
    match p.motion {
        Some(motion) => Payload::InputMotion(state, motion),
        None => Payload::Input(state),
    }
}

/// Sela e envia; devolve `false` se o motor deve parar (`reason` diz por quê).
fn transmit(
    shared: &Shared,
    socket: &UdpSocket,
    payload: &Payload,
    buf: &mut [u8; MAX_PACKET_LEN],
) -> Result<(), &'static str> {
    let len = match shared.session().send(payload, buf) {
        Ok(len) => len,
        Err(ProtoError::SeqExhausted) => return Err("seq_exhausted"),
        // Qualquer outro erro de envio é um bug de programação, não da rede: não derruba a sessão.
        Err(_) => return Ok(()),
    };
    // Falhas transitórias (rede indo e voltando) são ignoradas: o próximo pacote repara.
    let _ = socket.send(&buf[..len]);
    Ok(())
}

fn send_loop(shared: &Shared, socket: &UdpSocket, min_gap: Duration, listener: &dyn PumpListener) {
    let mut buf = [0u8; MAX_PACKET_LEN];
    let mut last_send = Instant::now()
        .checked_sub(min_gap)
        .unwrap_or_else(Instant::now);
    let mut last_ping = Instant::now();
    loop {
        // Espera uma mudança, ou o tempo do keepalive/ping, o que vier primeiro.
        let snapshot = {
            let mut p = shared.pending();
            if !p.dirty && !shared.stop.load(Ordering::SeqCst) {
                let wait = KEEPALIVE.saturating_sub(last_send.elapsed());
                p = shared
                    .wake
                    .wait_timeout(p, wait)
                    .unwrap_or_else(PoisonError::into_inner)
                    .0;
            }
            if shared.stop.load(Ordering::SeqCst) {
                return;
            }
            p.dirty = false;
            payload(&p, shared.now_us())
        };

        // Teto de taxa: se mudou rápido demais, espera o resto do intervalo e manda o estado mais novo.
        let since = last_send.elapsed();
        let snapshot = if since < min_gap {
            std::thread::sleep(min_gap.saturating_sub(since));
            if shared.stop.load(Ordering::SeqCst) {
                return;
            }
            let mut p = shared.pending();
            p.dirty = false;
            payload(&p, shared.now_us())
        } else {
            snapshot
        };

        if let Err(reason) = transmit(shared, socket, &snapshot, &mut buf) {
            shared.finish(listener, reason);
            return;
        }
        last_send = Instant::now();

        if last_ping.elapsed() >= PING_EVERY {
            let ping = Payload::Ping(Ping {
                t_client_us: shared.now_us(),
            });
            if let Err(reason) = transmit(shared, socket, &ping, &mut buf) {
                shared.finish(listener, reason);
                return;
            }
            last_ping = Instant::now();
        }
    }
}

fn receive_loop(shared: &Shared, socket: &UdpSocket, listener: &dyn PumpListener) {
    let mut buf = [0u8; MAX_PACKET_LEN * 2];
    loop {
        if shared.stop.load(Ordering::SeqCst) {
            return;
        }
        // Erro de leitura (tempo esgotado ou ICMP de porta fechada com o server ainda subindo): segue.
        if let Ok(len) = socket.recv(&mut buf) {
            // Datagrama inválido (replay, adulterado, de outra sessão) é descartado em silêncio.
            let packet = shared.session().receive(&buf[..len]);
            match packet.map(|p| p.payload) {
                Ok(Payload::Pong(pong)) => {
                    let now = shared.now_us();
                    let total = now.saturating_sub(pong.t_client_us).max(0);
                    let net = total.saturating_sub(i64::from(pong.server_proc_us));
                    listener.on_rtt(u32::try_from(net).unwrap_or(u32::MAX));
                }
                Ok(Payload::Rumble(rumble)) => {
                    {
                        // O próximo input confirma ao server que a vibração foi aplicada.
                        let mut p = shared.pending();
                        p.rumble_ack_id = rumble.rumble_id;
                        p.dirty = true;
                    }
                    shared.wake.notify_one();
                    listener.on_rumble(rumble.rumble_id, rumble.large, rumble.small);
                }
                _ => {}
            }
        }
    }
}
