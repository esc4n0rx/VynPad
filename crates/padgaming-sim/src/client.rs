//! O client simulado: envia input a uma taxa fixa, um Ping a cada 250 ms e mede o RTT.

use std::io::{self, ErrorKind};
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::Context;
use padgaming_protocol::{KEY_LEN, MAX_PACKET_LEN, Payload, Ping, Role, Secret, Session};

use crate::link::{FaultConfig, Link};
use crate::source::InputSource;

/// Intervalo entre Pings (PROTOCOLO §9).
const PING_INTERVAL: Duration = Duration::from_millis(250);
/// Espera ativa nos últimos instantes antes do próximo tick (o sleep do SO erra por ~1 ms).
const SPIN_MARGIN: Duration = Duration::from_millis(1);
/// Intervalo do evento de progresso.
const STATUS_INTERVAL: Duration = Duration::from_secs(1);

/// Como o client se comporta.
#[derive(Debug, Clone, Copy)]
pub struct ClientConfig {
    /// Endereço UDP do server.
    pub server: SocketAddr,
    /// `session_id` da sessão.
    pub session_id: u32,
    /// Chave da sessão (`udp_key`).
    pub key: [u8; KEY_LEN],
    /// Pacotes de input por segundo.
    pub rate_hz: u32,
    /// Falhas de rede a simular.
    pub fault: FaultConfig,
    /// Semente das falhas aleatórias.
    pub seed: u64,
}

/// Algo que vale mostrar ao usuário.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// O server mandou vibração.
    Rumble {
        /// Motor grande.
        large: u8,
        /// Motor pequeno.
        small: u8,
    },
    /// Resumo de progresso, uma vez por segundo.
    Status {
        /// Tempo desde o início.
        elapsed: Duration,
        /// Pacotes de input gerados até agora.
        inputs_sent: u64,
        /// Pongs recebidos até agora.
        pongs: u64,
        /// RTT do último Pong, em µs.
        last_rtt_us: Option<u32>,
    },
}

/// O que aconteceu durante a simulação.
#[derive(Debug, Clone, Default)]
pub struct ClientReport {
    /// Pacotes de input gerados.
    pub inputs_sent: u64,
    /// Pacotes perdidos pela falha simulada (input e ping).
    pub lost_by_fault: u64,
    /// Pings enviados.
    pub pings_sent: u64,
    /// Pongs recebidos.
    pub pongs: u64,
    /// RTT de cada Pong, em µs.
    pub rtt_us: Vec<u32>,
    /// `server_proc_us` de cada Pong.
    pub server_proc_us: Vec<u32>,
    /// Pacotes de vibração recebidos.
    pub rumbles: u64,
    /// Datagramas recebidos que não passaram na validação.
    pub bad_incoming: u64,
    /// Duração real.
    pub elapsed: Duration,
}

fn incoming<'a>(
    start: Instant,
    session: &'a mut Session,
    report: &'a mut ClientReport,
    last_rumble_id: &'a mut u16,
    on_event: &'a mut dyn FnMut(Event),
) -> Incoming<'a> {
    Incoming {
        session,
        start,
        report,
        last_rumble_id,
        on_event,
    }
}

fn micros(since: Instant, now: Instant) -> i64 {
    i64::try_from(now.saturating_duration_since(since).as_micros()).unwrap_or(i64::MAX)
}

/// Roda o client até a fonte acabar, `duration` passar ou `stop` ser ligado.
#[allow(clippy::too_many_lines)] // um laço só: pacing, pings, status e espera
pub fn run(
    config: &ClientConfig,
    source: &mut dyn InputSource,
    stop: &AtomicBool,
    duration: Option<Duration>,
    on_event: &mut dyn FnMut(Event),
) -> anyhow::Result<ClientReport> {
    anyhow::ensure!(config.rate_hz > 0, "a taxa deve ser maior que zero");
    let bind = if config.server.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    };
    let socket = UdpSocket::bind(bind).context("não consegui abrir o socket UDP")?;
    socket.set_nonblocking(true)?;
    let mut link = Link::new(socket, config.server, config.fault, config.seed);
    let mut session = Session::new(Role::Client, config.session_id, &Secret::new(config.key));

    let period = Duration::from_secs_f64(1.0 / f64::from(config.rate_hz));
    let start = Instant::now();
    let (mut next_tick, mut next_ping) = (start, start);
    let mut next_status = start + STATUS_INTERVAL;
    let mut report = ClientReport::default();
    let mut last_rumble_id = 0u16;
    let mut out = [0u8; MAX_PACKET_LEN];
    let mut inbox = [0u8; 512];

    while !stop.load(Ordering::Relaxed) {
        let now = Instant::now();
        let elapsed = now.duration_since(start);
        if duration.is_some_and(|d| elapsed >= d) {
            break;
        }

        if now >= next_tick {
            let Some(frame) = source.frame(elapsed) else {
                break;
            };
            let mut state = frame.state;
            state.t_client_us = micros(start, now);
            state.rumble_ack_id = last_rumble_id;
            let payload = match frame.motion {
                Some(motion) => Payload::InputMotion(state, motion),
                None => Payload::Input(state),
            };
            let n = session
                .send(&payload, &mut out)
                .context("sessão esgotada")?;
            link.submit(&out[..n], now)?;
            report.inputs_sent += 1;
            next_tick += period;
            // Se o processo travou por muito tempo, não tenta recuperar os ticks perdidos.
            if next_tick + period * 4 < now {
                next_tick = now + period;
            }
        }

        if now >= next_ping {
            let ping = Payload::Ping(Ping {
                t_client_us: micros(start, now),
            });
            let n = session.send(&ping, &mut out).context("sessão esgotada")?;
            link.submit(&out[..n], now)?;
            report.pings_sent += 1;
            next_ping += PING_INTERVAL;
        }

        link.flush(now)?;
        incoming(
            start,
            &mut session,
            &mut report,
            &mut last_rumble_id,
            &mut *on_event,
        )
        .drain(link.socket(), &mut inbox)?;

        if now >= next_status {
            on_event(Event::Status {
                elapsed,
                inputs_sent: report.inputs_sent,
                pongs: report.pongs,
                last_rtt_us: report.rtt_us.last().copied(),
            });
            next_status += STATUS_INTERVAL;
        }

        // Dorme até perto do próximo tick, e gira o resto para acertar o instante.
        let wait = next_tick.saturating_duration_since(Instant::now());
        if wait > SPIN_MARGIN * 2 {
            // Bloqueia no socket: uma resposta que chega acorda o laço na hora.
            incoming(
                start,
                &mut session,
                &mut report,
                &mut last_rumble_id,
                &mut *on_event,
            )
            .wait(link.socket(), &mut inbox, wait.saturating_sub(SPIN_MARGIN))?;
        } else {
            std::hint::spin_loop();
        }
    }

    // Dá uma última chance aos pacotes ainda agendados e às respostas em trânsito.
    let grace = Instant::now() + Duration::from_millis(50);
    while Instant::now() < grace {
        link.flush(Instant::now())?;
        incoming(
            start,
            &mut session,
            &mut report,
            &mut last_rumble_id,
            &mut *on_event,
        )
        .wait(link.socket(), &mut inbox, Duration::from_millis(2))?;
    }
    report.lost_by_fault = link.lost;
    report.elapsed = start.elapsed();
    Ok(report)
}

/// Estado que o tratamento de datagramas recebidos atualiza.
struct Incoming<'a> {
    session: &'a mut Session,
    start: Instant,
    report: &'a mut ClientReport,
    last_rumble_id: &'a mut u16,
    on_event: &'a mut dyn FnMut(Event),
}

impl Incoming<'_> {
    /// Trata um datagrama, medindo o RTT no instante `at` em que ele foi lido.
    fn handle(&mut self, datagram: &[u8], at: Instant) {
        let Ok(packet) = self.session.receive(datagram) else {
            self.report.bad_incoming += 1;
            return;
        };
        match packet.payload {
            Payload::Pong(pong) => {
                let rtt = micros(self.start, at) - pong.t_client_us;
                self.report.pongs += 1;
                self.report
                    .rtt_us
                    .push(u32::try_from(rtt.max(0)).unwrap_or(u32::MAX));
                self.report.server_proc_us.push(pong.server_proc_us);
            }
            Payload::Rumble(rumble) => {
                *self.last_rumble_id = rumble.rumble_id;
                self.report.rumbles += 1;
                (self.on_event)(Event::Rumble {
                    large: rumble.large,
                    small: rumble.small,
                });
            }
            _ => self.report.bad_incoming += 1,
        }
    }

    /// Lê tudo o que já chegou, sem bloquear.
    fn drain(&mut self, socket: &UdpSocket, inbox: &mut [u8; 512]) -> io::Result<()> {
        loop {
            match socket.recv_from(inbox) {
                Ok((n, _)) => self.handle(&inbox[..n], Instant::now()),
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(()),
                // ICMP de um envio anterior (server fora do ar): não é um erro do client.
                Err(e) if e.kind() == ErrorKind::ConnectionReset => {}
                Err(e) => return Err(e),
            }
        }
    }

    /// Espera até `timeout` por um datagrama e o trata no instante exato em que chega, para o
    /// RTT não incluir o intervalo de sono do laço principal.
    fn wait(
        &mut self,
        socket: &UdpSocket,
        inbox: &mut [u8; 512],
        timeout: Duration,
    ) -> io::Result<()> {
        // O timeout do socket tem granularidade de 1 ms e zero significaria "sem limite".
        let timeout = timeout.max(Duration::from_millis(1));
        socket.set_nonblocking(false)?;
        socket.set_read_timeout(Some(timeout))?;
        let received = socket.recv_from(inbox);
        let at = Instant::now();
        socket.set_nonblocking(true)?;
        match received {
            Ok((n, _)) => {
                self.handle(&inbox[..n], at);
                Ok(())
            }
            Err(e)
                if matches!(
                    e.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::ConnectionReset
                ) =>
            {
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}
