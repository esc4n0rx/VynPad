//! Fallback de descoberta por broadcast UDP (PROTOCOLO §3.2): responde `VPDISC1` com um JSON.

use std::io::{self, ErrorKind};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use socket2::{Domain, Protocol, Socket, Type};
use vynpad_protocol::discovery::{BROADCAST_PORT, BROADCAST_REQUEST};

use super::info::ServerInfo;

/// Respostas por segundo, no máximo. A resposta é bem maior que o pedido de 7 bytes; sem teto o
/// server viraria um amplificador para quem forjar a origem.
pub const MAX_REPLIES_PER_SECOND: u32 = 32;
/// De quanto em quanto tempo a thread confere o pedido de parada.
const POLL: Duration = Duration::from_millis(100);

/// Janela fixa de 1 s com um teto de respostas.
#[derive(Debug)]
pub struct RateLimiter {
    window_start: Instant,
    count: u32,
    max: u32,
}

impl RateLimiter {
    /// Teto de `max` respostas por segundo, começando em `now`.
    pub const fn new(max: u32, now: Instant) -> Self {
        Self {
            window_start: now,
            count: 0,
            max,
        }
    }

    /// Pode responder agora?
    pub fn allow(&mut self, now: Instant) -> bool {
        if now.saturating_duration_since(self.window_start) >= Duration::from_secs(1) {
            self.window_start = now;
            self.count = 0;
        }
        if self.count < self.max {
            self.count += 1;
            true
        } else {
            false
        }
    }
}

/// O respondedor em execução.
pub struct BroadcastResponder {
    stop: Arc<AtomicBool>,
    local_addr: SocketAddr,
    thread: Option<JoinHandle<()>>,
}

impl BroadcastResponder {
    /// Endereço padrão de escuta: `0.0.0.0:47802`.
    pub const fn default_bind() -> SocketAddr {
        SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::UNSPECIFIED), BROADCAST_PORT)
    }

    /// Abre o socket e inicia a thread `discovery-bcast`. `info` é lido a cada pedido, então
    /// mudanças (slots livres) valem na hora.
    pub fn start(bind: SocketAddr, info: Arc<ArcSwap<ServerInfo>>) -> io::Result<Self> {
        let socket = Socket::new(Domain::for_address(bind), Type::DGRAM, Some(Protocol::UDP))?;
        // Outro app (ou uma segunda instância morrendo) pode estar na porta.
        socket.set_reuse_address(true)?;
        socket.bind(&bind.into())?;
        let socket = UdpSocket::from(socket);
        socket.set_read_timeout(Some(POLL))?;
        let local_addr = socket.local_addr()?;

        let stop = Arc::new(AtomicBool::new(false));
        let thread = std::thread::Builder::new()
            .name("discovery-bcast".into())
            .spawn({
                let stop = Arc::clone(&stop);
                move || run(&socket, &info, &stop)
            })?;
        tracing::info!(%local_addr, "respondedor de broadcast VPDISC1 iniciado");
        Ok(Self {
            stop,
            local_addr,
            thread: Some(thread),
        })
    }

    /// Endereço em que o socket escuta (útil com a porta 0, nos testes).
    pub const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Pede a parada e espera a thread (em até ~100 ms).
    pub fn shutdown(mut self) {
        self.stop_and_join();
    }

    fn stop_and_join(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for BroadcastResponder {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

fn run(socket: &UdpSocket, info: &ArcSwap<ServerInfo>, stop: &AtomicBool) {
    // Maior que o pedido; um datagrama maior é cortado e, por ter `len` diferente, ignorado.
    let mut buf = [0u8; 64];
    let mut limiter = RateLimiter::new(MAX_REPLIES_PER_SECOND, Instant::now());
    while !stop.load(Ordering::Relaxed) {
        let (len, from) = match socket.recv_from(&mut buf) {
            Ok(received) => received,
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => continue,
            // O Windows devolve isto quando um destino anterior recusou o nosso unicast.
            Err(e) if e.kind() == ErrorKind::ConnectionReset => continue,
            Err(error) => {
                tracing::warn!(%error, "falha ao ler o broadcast de descoberta");
                std::thread::sleep(POLL);
                continue;
            }
        };
        if len != BROADCAST_REQUEST.len() || buf[..len] != BROADCAST_REQUEST[..] {
            continue; // Qualquer outra coisa é ignorada (§3.2).
        }
        if !limiter.allow(Instant::now()) {
            tracing::debug!("VPDISC1 acima do limite: ignorado");
            continue;
        }
        let reply = info.load().response().to_json();
        if let Err(error) = socket.send_to(&reply, from) {
            tracing::debug!(%error, "não consegui responder ao VPDISC1");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::UdpSocket;

    use vynpad_protocol::discovery::DiscoveryResponse;

    use super::super::info::tests::sample;
    use super::*;

    fn start() -> (BroadcastResponder, Arc<ArcSwap<ServerInfo>>) {
        let info = Arc::new(ArcSwap::from_pointee(sample()));
        let responder =
            BroadcastResponder::start("127.0.0.1:0".parse().unwrap(), Arc::clone(&info)).unwrap();
        (responder, info)
    }

    fn client() -> UdpSocket {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(500)))
            .unwrap();
        socket
    }

    #[test]
    fn pgdisc1_gets_a_unicast_json_reply_that_follows_the_schema() {
        let (responder, _) = start();
        let client = client();
        client.send_to(b"VPDISC1", responder.local_addr()).unwrap();
        let mut buf = [0u8; 512];
        let (n, from) = client.recv_from(&mut buf).unwrap();
        assert_eq!(from, responder.local_addr());
        let response = DiscoveryResponse::from_json(&buf[..n]).unwrap();
        assert_eq!(response, sample().response());
    }

    #[test]
    fn other_datagrams_are_ignored() {
        let (responder, _) = start();
        let client = client();
        for junk in [
            &b"VPDISC2"[..],
            b"pgdisc1",
            b"VPDISC1 ",
            b"",
            b"GET / HTTP/1.1",
        ] {
            client.send_to(junk, responder.local_addr()).unwrap();
        }
        let mut buf = [0u8; 512];
        assert!(client.recv_from(&mut buf).is_err(), "não deveria responder");
        // E continua respondendo ao pedido certo depois do lixo.
        client.send_to(b"VPDISC1", responder.local_addr()).unwrap();
        assert!(client.recv_from(&mut buf).is_ok());
    }

    #[test]
    fn the_reply_reflects_slot_changes_immediately() {
        let (responder, info) = start();
        let client = client();
        let mut buf = [0u8; 512];
        for free in [3u8, 0] {
            let mut next = (**info.load()).clone();
            next.free = free;
            info.store(Arc::new(next));
            client.send_to(b"VPDISC1", responder.local_addr()).unwrap();
            let (n, _) = client.recv_from(&mut buf).unwrap();
            assert_eq!(DiscoveryResponse::from_json(&buf[..n]).unwrap().free, free);
        }
    }

    #[test]
    fn a_flood_gets_at_most_the_per_second_cap() {
        let (responder, _) = start();
        let client = client();
        for _ in 0..200 {
            client.send_to(b"VPDISC1", responder.local_addr()).unwrap();
        }
        let mut buf = [0u8; 512];
        let mut replies = 0;
        while client.recv_from(&mut buf).is_ok() {
            replies += 1;
        }
        // 200 pedidos em ~ms: no máximo uma janela cheia (e talvez o começo da seguinte).
        assert!(replies >= 1);
        assert!(
            replies <= 2 * MAX_REPLIES_PER_SECOND as usize,
            "{replies} respostas"
        );
    }

    #[test]
    fn the_limiter_window_resets_after_a_second() {
        let t0 = Instant::now();
        let mut limiter = RateLimiter::new(2, t0);
        assert!(limiter.allow(t0));
        assert!(limiter.allow(t0));
        assert!(!limiter.allow(t0 + Duration::from_millis(999)));
        assert!(limiter.allow(t0 + Duration::from_secs(1)));
    }
}
