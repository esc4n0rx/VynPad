//! Descoberta por broadcast `VPDISC1` (PROTOCOLO §3.2).
//!
//! O mDNS fica a cargo da plataforma (no Android, o `NsdManager`); este scanner é o fallback e
//! entrega cada server assim que ele responde, para a UI desenhar o radar em tempo real.

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use vynpad_protocol::discovery::{BROADCAST_PORT, BROADCAST_REQUEST, DiscoveryResponse};

use crate::{CoreError, DiscoveredServer};

/// Intervalo entre os envios do pedido (o UDP perde pacotes; repetir é barato).
const RESEND_EVERY: Duration = Duration::from_millis(1000);
/// Quanto cada leitura espera antes de checar o cancelamento.
const POLL: Duration = Duration::from_millis(100);

/// Quem recebe os servers conforme eles respondem. Implementado pelo Kotlin/Swift.
#[uniffi::export(with_foreign)]
pub trait ScanListener: Send + Sync {
    /// Um server respondeu (uma vez por server e endereço de origem).
    fn on_server(&self, server: DiscoveredServer);
}

/// Procura servers por broadcast UDP.
#[derive(uniffi::Object)]
pub struct BroadcastScanner {
    targets: Vec<SocketAddr>,
    cancelled: AtomicBool,
}

impl BroadcastScanner {
    /// Scanner com destinos explícitos (usado nos testes, com um server em loopback).
    #[must_use]
    pub fn with_targets(targets: Vec<SocketAddr>) -> Arc<Self> {
        Arc::new(Self {
            targets,
            cancelled: AtomicBool::new(false),
        })
    }
}

#[uniffi::export]
impl BroadcastScanner {
    /// `extra_broadcast`: endereços de broadcast das interfaces (ex.: `192.168.0.255`), além do
    /// limitado `255.255.255.255`. Valores que não forem IPv4 são ignorados.
    #[uniffi::constructor]
    pub fn new(extra_broadcast: &[String]) -> Arc<Self> {
        let mut targets = vec![SocketAddr::from((Ipv4Addr::BROADCAST, BROADCAST_PORT))];
        targets.extend(
            extra_broadcast
                .iter()
                .filter_map(|a| a.parse::<Ipv4Addr>().ok())
                .map(|ip| SocketAddr::from((ip, BROADCAST_PORT))),
        );
        targets.sort();
        targets.dedup();
        Self::with_targets(targets)
    }

    /// Procura por `wait_ms` (ou até [`cancel`](Self::cancel)), repetindo o pedido a cada 1 s.
    /// Bloqueia a thread que chamou: rode fora da thread principal.
    // O UniFFI entrega o listener do Kotlin/Swift como `Arc` por valor.
    #[allow(clippy::needless_pass_by_value)]
    pub fn scan(&self, wait_ms: u32, listener: Arc<dyn ScanListener>) -> Result<(), CoreError> {
        self.cancelled.store(false, Ordering::SeqCst);
        let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| io_error(&e))?;
        socket.set_broadcast(true).map_err(|e| io_error(&e))?;
        socket
            .set_read_timeout(Some(POLL))
            .map_err(|e| io_error(&e))?;

        let deadline = Instant::now() + Duration::from_millis(u64::from(wait_ms));
        let mut next_send = Instant::now();
        let mut seen: HashSet<(String, IpAddr)> = HashSet::new();
        let mut buf = [0u8; 1024];

        while Instant::now() < deadline && !self.cancelled.load(Ordering::SeqCst) {
            if Instant::now() >= next_send {
                for target in &self.targets {
                    // Uma interface sem rota falha; as outras seguem.
                    let _ = socket.send_to(BROADCAST_REQUEST, target);
                }
                next_send += RESEND_EVERY;
            }
            let Ok((len, from)) = socket.recv_from(&mut buf) else {
                continue; // timeout de leitura
            };
            // A resposta é só uma dica: JSON inválido é ignorado, a confiança vem do pinning.
            let Ok(response) = DiscoveryResponse::from_json(&buf[..len]) else {
                continue;
            };
            if !seen.insert((response.id.as_str().to_owned(), from.ip())) {
                continue;
            }
            listener.on_server(DiscoveredServer {
                id: response.id.as_str().to_owned(),
                name: response.name,
                protocol_version: response.ver,
                tcp_port: response.tcp,
                udp_port: response.udp,
                os: response.os,
                free_slots: response.free,
                short_fingerprint: response.fp,
                hosts: vec![from.ip().to_string()],
            });
        }
        Ok(())
    }

    /// Interrompe um `scan` em andamento (ele retorna em até ~100 ms).
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
}

fn io_error(e: &std::io::Error) -> CoreError {
    CoreError::InvalidArgument {
        reason: format!("rede indisponível: {e}"),
    }
}
