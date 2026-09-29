//! Enlace com falhas simuladas: perda, atraso, jitter e reordenação dos pacotes enviados.
//!
//! As falhas valem para o sentido client → server (o que carrega o input). Um pacote
//! reordenado chega depois de um mais novo e o server o descarta como `replay`, que é
//! justamente o que se quer observar.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use crate::units::Rng;

/// Extra de atraso dado a um pacote "reordenado" (chega depois de vários mais novos).
const REORDER_EXTRA: Duration = Duration::from_millis(20);
/// Fração dos pacotes que `--reorder` atrasa.
const REORDER_FRACTION: f64 = 0.1;

/// Falhas de rede a simular.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FaultConfig {
    /// Probabilidade de perder um pacote (0 a 1).
    pub loss: f64,
    /// Atraso fixo somado a todo pacote.
    pub delay: Duration,
    /// Atraso extra aleatório, de 0 até este valor.
    pub jitter: Duration,
    /// Atrasa parte dos pacotes para chegarem fora de ordem.
    pub reorder: bool,
}

impl FaultConfig {
    /// Nenhuma falha simulada.
    pub fn is_clean(&self) -> bool {
        self.loss == 0.0 && self.delay.is_zero() && self.jitter.is_zero() && !self.reorder
    }
}

struct Pending {
    due: Instant,
    order: u64,
    packet: Vec<u8>,
}

impl PartialEq for Pending {
    fn eq(&self, other: &Self) -> bool {
        (self.due, self.order) == (other.due, other.order)
    }
}
impl Eq for Pending {}
impl PartialOrd for Pending {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Pending {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.due, self.order).cmp(&(other.due, other.order))
    }
}

/// Envia pacotes UDP aplicando as falhas configuradas.
pub struct Link {
    socket: UdpSocket,
    target: SocketAddr,
    config: FaultConfig,
    rng: Rng,
    queue: BinaryHeap<Reverse<Pending>>,
    counter: u64,
    /// Pacotes que a perda simulada descartou.
    pub lost: u64,
    /// Pacotes entregues ao socket.
    pub sent: u64,
}

impl Link {
    /// Cria o enlace sobre `socket`, enviando para `target`.
    pub fn new(socket: UdpSocket, target: SocketAddr, config: FaultConfig, seed: u64) -> Self {
        Self {
            socket,
            target,
            config,
            rng: Rng::new(seed),
            queue: BinaryHeap::new(),
            counter: 0,
            lost: 0,
            sent: 0,
        }
    }

    /// O socket (para receber Pong e Rumble).
    pub const fn socket(&self) -> &UdpSocket {
        &self.socket
    }

    /// Entrega o pacote agora, ou o agenda conforme as falhas.
    pub fn submit(&mut self, packet: &[u8], now: Instant) -> io::Result<()> {
        if self.rng.chance(self.config.loss) {
            self.lost += 1;
            return Ok(());
        }
        let mut delay = self.config.delay;
        if !self.config.jitter.is_zero() {
            delay += self.config.jitter.mul_f64(self.rng.unit());
        }
        if self.config.reorder && self.rng.chance(REORDER_FRACTION) {
            delay += REORDER_EXTRA;
        }
        if delay.is_zero() && self.queue.is_empty() {
            return self.send_now(packet);
        }
        self.counter += 1;
        self.queue.push(Reverse(Pending {
            due: now + delay,
            order: self.counter,
            packet: packet.to_vec(),
        }));
        Ok(())
    }

    /// Envia os pacotes agendados cujo prazo chegou.
    pub fn flush(&mut self, now: Instant) -> io::Result<()> {
        while self.queue.peek().is_some_and(|p| p.0.due <= now) {
            let Reverse(pending) = self.queue.pop().expect("conferido por peek");
            self.send_now(&pending.packet)?;
        }
        Ok(())
    }

    /// Pacotes ainda agendados.
    pub fn in_flight(&self) -> usize {
        self.queue.len()
    }

    fn send_now(&mut self, packet: &[u8]) -> io::Result<()> {
        // `ConnectionReset` é o ICMP de um envio anterior (server fora do ar); não é fatal.
        match self.socket.send_to(packet, self.target) {
            Ok(_) => {
                self.sent += 1;
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::ConnectionReset => Ok(()),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Um par de sockets em loopback: o enlace envia, o `receiver` recebe.
    fn pair(config: FaultConfig) -> (Link, UdpSocket) {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver.set_nonblocking(true).unwrap();
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        let link = Link::new(sender, receiver.local_addr().unwrap(), config, 1);
        (link, receiver)
    }

    fn drain(receiver: &UdpSocket) -> Vec<u8> {
        let mut got = Vec::new();
        let mut buf = [0u8; 16];
        while let Ok((n, _)) = receiver.recv_from(&mut buf) {
            got.push(buf[..n][0]);
        }
        got
    }

    #[test]
    fn a_clean_link_delivers_immediately_and_in_order() {
        let (mut link, rx) = pair(FaultConfig::default());
        assert!(FaultConfig::default().is_clean());
        let now = Instant::now();
        for i in 0..5u8 {
            link.submit(&[i], now).unwrap();
        }
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(drain(&rx), [0, 1, 2, 3, 4]);
        assert_eq!((link.sent, link.lost, link.in_flight()), (5, 0, 0));
    }

    #[test]
    fn loss_drops_roughly_the_configured_fraction() {
        let (mut link, _rx) = pair(FaultConfig {
            loss: 0.3,
            ..FaultConfig::default()
        });
        let now = Instant::now();
        for _ in 0..2000 {
            link.submit(&[0], now).unwrap();
        }
        assert!((500..700).contains(&(link.lost as i64)), "{}", link.lost);
        assert_eq!(link.sent + link.lost, 2000);
    }

    #[test]
    fn total_loss_delivers_nothing() {
        let (mut link, rx) = pair(FaultConfig {
            loss: 1.0,
            ..FaultConfig::default()
        });
        link.submit(&[1], Instant::now()).unwrap();
        std::thread::sleep(Duration::from_millis(10));
        assert!(drain(&rx).is_empty());
    }

    #[test]
    fn delay_holds_packets_until_their_time() {
        let cfg = FaultConfig {
            delay: Duration::from_millis(50),
            ..FaultConfig::default()
        };
        assert!(!cfg.is_clean());
        let (mut link, rx) = pair(cfg);
        let start = Instant::now();
        link.submit(&[7], start).unwrap();
        link.flush(start + Duration::from_millis(49)).unwrap();
        std::thread::sleep(Duration::from_millis(10));
        assert!(drain(&rx).is_empty());
        assert_eq!(link.in_flight(), 1);
        link.flush(start + Duration::from_millis(50)).unwrap();
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(drain(&rx), [7]);
    }

    #[test]
    fn jitter_stays_within_bounds_and_can_reorder_neighbours() {
        let cfg = FaultConfig {
            jitter: Duration::from_millis(30),
            ..FaultConfig::default()
        };
        let (mut link, rx) = pair(cfg);
        let start = Instant::now();
        for i in 0..50u8 {
            link.submit(&[i], start + Duration::from_millis(u64::from(i)))
                .unwrap();
        }
        link.flush(start + Duration::from_millis(50 + 30)).unwrap();
        std::thread::sleep(Duration::from_millis(20));
        let got = drain(&rx);
        assert_eq!(got.len(), 50, "nada se perde; só chega em outra ordem");
        assert_ne!(
            got,
            (0..50u8).collect::<Vec<_>>(),
            "o jitter deveria ter misturado a ordem"
        );
    }

    #[test]
    fn reorder_delays_a_fraction_of_packets_past_newer_ones() {
        let cfg = FaultConfig {
            reorder: true,
            ..FaultConfig::default()
        };
        let (mut link, rx) = pair(cfg);
        let start = Instant::now();
        for i in 0..100u8 {
            link.submit(&[i], start + Duration::from_millis(u64::from(i)))
                .unwrap();
        }
        link.flush(start + Duration::from_secs(1)).unwrap();
        std::thread::sleep(Duration::from_millis(30));
        let got = drain(&rx);
        assert_eq!(got.len(), 100);
        let inversions = got.windows(2).filter(|w| w[0] > w[1]).count();
        assert!(inversions > 0, "esperava pacotes fora de ordem");
    }
}
