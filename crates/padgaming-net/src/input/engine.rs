//! O núcleo do loop de input, sem socket: um datagrama entra, o `XReport` sai no driver.
//!
//! Separado do `UdpSocket` para poder ser testado (e medido) sem rede. Por datagrama:
//! cabeçalho → sessão pelo `session_id` (tabela fixa) → `ReplayWindow` e AEAD → pipeline de
//! mapeamento → `SlotManager::submit`. Nada aloca.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use arc_swap::ArcSwapOption;
use padgaming_core::SlotManager;
use padgaming_mapping::{Mapper, ProfileCell, RawInput};
use padgaming_protocol::{
    KEY_LEN, MAX_PACKET_LEN, Payload, Pong, ProtoError, Role, Secret, Session, parse_header,
};

use super::metrics::SessionMetrics;
use super::stats::InputStats;

/// Sessões simultâneas que a tabela comporta (4 slots, mais folga para a troca de sessão numa
/// reconexão, quando a antiga ainda não foi removida).
pub const MAX_SESSIONS: usize = 8;

/// Quando a sessão foi vista pela última vez e de onde (para os timeouts da T-304).
#[derive(Debug)]
pub struct SessionActivity {
    epoch: Instant,
    last_seen_us: AtomicU64,
    peer: ArcSwapOption<SocketAddr>,
    /// Qualidade da conexão: perda, jitter, RTT, bateria e tempo de processamento.
    pub metrics: SessionMetrics,
}

impl SessionActivity {
    fn new(epoch: Instant, now: Instant) -> Self {
        let activity = Self {
            epoch,
            last_seen_us: AtomicU64::new(0),
            peer: ArcSwapOption::empty(),
            metrics: SessionMetrics::default(),
        };
        activity.touch(now);
        activity
    }

    fn touch(&self, now: Instant) {
        let us = u64::try_from(now.saturating_duration_since(self.epoch).as_micros())
            .unwrap_or(u64::MAX);
        self.last_seen_us.store(us, Ordering::Relaxed);
    }

    /// Há quanto tempo chegou o último pacote autêntico da sessão.
    pub fn idle_for(&self, now: Instant) -> Duration {
        let last = self.epoch + Duration::from_micros(self.last_seen_us.load(Ordering::Relaxed));
        now.saturating_duration_since(last)
    }

    /// Último endereço de origem de um pacote autêntico (acompanha troca de IP/porta).
    pub fn peer(&self) -> Option<SocketAddr> {
        self.peer.load_full().map(|addr| *addr)
    }
}

/// O que registrar para uma sessão nova.
pub struct SessionSpec {
    /// `session_id` do `welcome`.
    pub session_id: u32,
    /// Chave `udp_key` do `welcome`.
    pub key: Secret<[u8; KEY_LEN]>,
    /// Perfil de mapeamento do slot.
    pub profile: Arc<ProfileCell>,
}

/// Sessão dentro do motor.
pub struct SessionEntry {
    session: Session,
    mapper: Mapper,
    activity: Arc<SessionActivity>,
    /// Origem do último pacote autêntico (cópia local, para só publicar mudanças).
    peer: Option<SocketAddr>,
    last_input: Option<Instant>,
}

impl SessionEntry {
    /// Monta a sessão. Devolve também o acompanhamento de atividade.
    pub fn new(spec: SessionSpec, epoch: Instant) -> (Self, Arc<SessionActivity>) {
        let activity = Arc::new(SessionActivity::new(epoch, Instant::now()));
        let entry = Self {
            session: Session::new(Role::Server, spec.session_id, &spec.key),
            mapper: Mapper::new(spec.profile),
            activity: Arc::clone(&activity),
            peer: None,
            last_input: None,
        };
        (entry, activity)
    }
}

/// Resultado do processamento de um datagrama.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handled {
    /// Input autêntico entregue ao pipeline.
    Input,
    /// Ping: mande `reply_len` bytes de `out` para `to`.
    Pong {
        /// Bytes a enviar.
        reply_len: usize,
        /// Destino.
        to: SocketAddr,
    },
    /// Descartado (já contado em [`InputStats`]).
    Dropped,
}

/// Menor `dt` entre dois inputs quando não há histórico (primeiro pacote da sessão).
const FIRST_INPUT_DT: f32 = 0.004;

/// Estado do loop de input: sessões, slots, contadores.
pub struct InputEngine {
    sessions: [Option<SessionEntry>; MAX_SESSIONS],
    slots: Arc<SlotManager>,
    stats: Arc<InputStats>,
    epoch: Instant,
}

impl InputEngine {
    /// Cria o motor. `epoch` é a origem do relógio do server (`t_server_rx_us`).
    pub fn new(slots: Arc<SlotManager>, stats: Arc<InputStats>, epoch: Instant) -> Self {
        Self {
            sessions: Default::default(),
            slots,
            stats,
            epoch,
        }
    }

    /// Registra uma sessão; se já existir uma com o mesmo `session_id`, ela é substituída.
    /// Devolve `false` se a tabela estiver cheia.
    pub fn add_session(&mut self, entry: SessionEntry) -> bool {
        let id = entry.session.session_id();
        self.remove_session(id);
        match self.sessions.iter_mut().find(|s| s.is_none()) {
            Some(free) => {
                *free = Some(entry);
                true
            }
            None => false,
        }
    }

    /// Remove a sessão; pacotes dela passam a ser descartados como `unknown_session`.
    pub fn remove_session(&mut self, session_id: u32) {
        for slot in &mut self.sessions {
            if slot
                .as_ref()
                .is_some_and(|s| s.session.session_id() == session_id)
            {
                *slot = None;
            }
        }
    }

    /// Sessões registradas.
    pub fn session_count(&self) -> usize {
        self.sessions.iter().flatten().count()
    }

    /// Processa um datagrama recebido de `src` no instante `rx`.
    ///
    /// Respostas (Pong) são escritas em `out`; o chamador as envia conforme o [`Handled`].
    pub fn handle(
        &mut self,
        datagram: &[u8],
        src: SocketAddr,
        rx: Instant,
        out: &mut [u8; MAX_PACKET_LEN],
    ) -> Handled {
        self.stats.received.fetch_add(1, Ordering::Relaxed);

        let header = match parse_header(datagram, Role::Server) {
            Ok(header) => header,
            Err(error) => return Self::drop_for(&self.stats, error),
        };
        let Some(entry) = self
            .sessions
            .iter_mut()
            .flatten()
            .find(|s| s.session.session_id() == header.session_id)
        else {
            return Self::drop_for(&self.stats, ProtoError::UnknownSession);
        };
        let packet = match entry.session.receive_parsed(&header, datagram) {
            Ok(packet) => packet,
            Err(error) => {
                if error == ProtoError::StaleSeq {
                    entry.activity.metrics.on_stale();
                }
                return Self::drop_for(&self.stats, error);
            }
        };

        // Daqui em diante o pacote é autêntico e novo.
        entry.activity.touch(rx);
        entry.activity.metrics.on_packet(packet.seq);
        if entry.peer != Some(src) {
            entry.peer = Some(src);
            // Só na primeira vez e em roaming (troca de IP/porta): aloca, mas é raro.
            entry.activity.peer.store(Some(Arc::new(src)));
        }

        let handled = match packet.payload {
            Payload::Input(state) => {
                entry.activity.metrics.set_battery(state.battery);
                Self::deliver(
                    &self.slots,
                    &self.stats,
                    entry,
                    &RawInput::from_wire(&state, None),
                    rx,
                );
                Handled::Input
            }
            Payload::InputMotion(state, motion) => {
                entry.activity.metrics.set_battery(state.battery);
                Self::deliver(
                    &self.slots,
                    &self.stats,
                    entry,
                    &RawInput::from_wire(&state, Some(&motion)),
                    rx,
                );
                Handled::Input
            }
            Payload::Ping(request) => {
                let reply = Payload::Pong(Pong {
                    t_client_us: request.t_client_us,
                    t_server_rx_us: i64::try_from(
                        rx.saturating_duration_since(self.epoch).as_micros(),
                    )
                    .unwrap_or(i64::MAX),
                    server_proc_us: u32::try_from(rx.elapsed().as_micros()).unwrap_or(u32::MAX),
                });
                match entry.session.send(&reply, out) {
                    Ok(reply_len) => {
                        self.stats.pings.fetch_add(1, Ordering::Relaxed);
                        Handled::Pong { reply_len, to: src }
                    }
                    // A sessão esgotou o `seq` de envio: sem Pong, o client renova a sessão.
                    Err(_) => Handled::Dropped,
                }
            }
            // `parse_header` já recusa as direções erradas; este braço não é alcançável.
            Payload::Pong(_) | Payload::Rumble(_) => {
                return Self::drop_for(&self.stats, ProtoError::WrongDirection);
            }
        };
        if handled == Handled::Input {
            let elapsed = rx.elapsed();
            self.stats.processing.record(elapsed);
            entry.activity.metrics.record_processing(elapsed);
        }
        handled
    }

    fn deliver(
        slots: &SlotManager,
        stats: &InputStats,
        entry: &mut SessionEntry,
        raw: &RawInput,
        rx: Instant,
    ) {
        let dt = entry.last_input.map_or(FIRST_INPUT_DT, |last| {
            let interval = rx.saturating_duration_since(last);
            entry.activity.metrics.on_interval(interval);
            interval.as_secs_f32()
        });
        entry.last_input = Some(rx);
        let report = entry.mapper.map(raw, dt);
        match slots.submit(entry.session.session_id(), &report) {
            Ok(()) => {
                stats.inputs.fetch_add(1, Ordering::Relaxed);
            }
            Err(_) => {
                stats.submit_errors.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn drop_for(stats: &InputStats, error: ProtoError) -> Handled {
        let counter = match error {
            ProtoError::UnknownSession => &stats.dropped_unknown_session,
            ProtoError::BadTag => &stats.dropped_auth,
            ProtoError::StaleSeq => &stats.dropped_replay,
            _ => &stats.dropped_malformed,
        };
        counter.fetch_add(1, Ordering::Relaxed);
        Handled::Dropped
    }
}
