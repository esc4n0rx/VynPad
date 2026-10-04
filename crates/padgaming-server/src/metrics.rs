//! O coletor de métricas (T-305): lê as métricas de cada sessão viva, calcula taxa e perda numa
//! janela curta, classifica a qualidade e entrega uma foto por slot.
//!
//! O coletor roda fora do hot path (4 Hz): quem escreve as métricas é o loop de input e o
//! servidor de controle, sem lock; aqui só se lê.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use padgaming_driver::SlotId;
use padgaming_net::control::ControlServer;

/// Janela em que a perda e a taxa são calculadas.
const HISTORY: Duration = Duration::from_secs(10);
/// A taxa usa a amostra mais próxima deste tempo atrás (mais estável que a de 250 ms).
const RATE_SPAN: Duration = Duration::from_secs(1);

/// Como a conexão está, para os ícones e as cores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Quality {
    /// Sem atraso perceptível.
    Excellent,
    /// Boa para qualquer jogo.
    Good,
    /// Oscila: jogos rápidos podem sentir.
    Unstable,
    /// Atrapalha o jogo.
    Poor,
}

/// Até onde cada medida pode ir para a conexão ainda merecer uma classe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    /// RTT p95, em ms.
    pub rtt_p95_ms: f64,
    /// Jitter do intervalo entre inputs, em ms.
    pub jitter_ms: f64,
    /// Perda de pacotes, em %.
    pub loss_pct: f64,
}

/// Limiares da classificação (acima do último, a conexão é [`Quality::Poor`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QualityThresholds {
    /// Até aqui, [`Quality::Excellent`].
    pub excellent: Limits,
    /// Até aqui, [`Quality::Good`].
    pub good: Limits,
    /// Até aqui, [`Quality::Unstable`].
    pub unstable: Limits,
}

impl Default for QualityThresholds {
    fn default() -> Self {
        Self {
            excellent: Limits {
                rtt_p95_ms: 20.0,
                jitter_ms: 3.0,
                loss_pct: 0.1,
            },
            good: Limits {
                rtt_p95_ms: 40.0,
                jitter_ms: 8.0,
                loss_pct: 1.0,
            },
            unstable: Limits {
                rtt_p95_ms: 80.0,
                jitter_ms: 20.0,
                loss_pct: 5.0,
            },
        }
    }
}

impl QualityThresholds {
    /// A pior das três medidas decide a classe. Sem RTT ainda, só o jitter e a perda contam.
    #[must_use]
    pub fn classify(
        &self,
        rtt_p95: Option<Duration>,
        jitter: Duration,
        loss_ratio: f64,
    ) -> Quality {
        let rtt = rtt_p95.map_or(0.0, |d| d.as_secs_f64() * 1000.0);
        let jitter = jitter.as_secs_f64() * 1000.0;
        let loss = loss_ratio * 100.0;
        let within =
            |l: &Limits| rtt <= l.rtt_p95_ms && jitter <= l.jitter_ms && loss <= l.loss_pct;
        if within(&self.excellent) {
            Quality::Excellent
        } else if within(&self.good) {
            Quality::Good
        } else if within(&self.unstable) {
            Quality::Unstable
        } else {
            Quality::Poor
        }
    }
}

/// As métricas de um slot com sessão.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotMetrics {
    /// Slot (0–3).
    pub slot: SlotId,
    /// Sessão que está nele.
    pub session_id: u32,
    /// Último RTT medido.
    pub rtt_last: Option<Duration>,
    /// RTT p50/p95/p99 dos últimos 60 s.
    pub rtt_p: [Duration; 3],
    /// Jitter do intervalo entre inputs.
    pub jitter: Duration,
    /// Fração de pacotes perdidos nos últimos 10 s (0–1).
    pub loss_ratio: f64,
    /// Pacotes repetidos ou fora de ordem desde o início da sessão.
    pub out_of_order: u64,
    /// Pacotes por segundo.
    pub rate_hz: f64,
    /// Processamento no server p50/p95/p99 dos últimos 60 s.
    pub processing_p: [Duration; 3],
    /// Bateria do celular (0–100).
    pub battery: Option<u8>,
    /// Classe da conexão; `None` até chegar o primeiro pacote.
    pub quality: Option<Quality>,
}

/// As métricas de todos os slots com sessão.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MetricsSnapshot {
    /// Um item por slot com sessão viva, na ordem dos slots.
    pub slots: Vec<SlotMetrics>,
}

#[derive(Debug, Default)]
struct History {
    /// `(quando, recebidos, perdidos)`, do mais antigo ao mais novo.
    samples: VecDeque<(Instant, u64, u64)>,
}

impl History {
    fn push(&mut self, now: Instant, received: u64, lost: u64) {
        self.samples.push_back((now, received, lost));
        while self
            .samples
            .front()
            .is_some_and(|(at, ..)| now.saturating_duration_since(*at) > HISTORY)
        {
            self.samples.pop_front();
        }
    }

    /// `(taxa em pacotes/s, fração perdida)`.
    #[allow(clippy::cast_precision_loss)] // contadores muito abaixo de 2^52
    fn rates(&self, now: Instant) -> (f64, f64) {
        let Some(&(_, newest_rx, newest_lost)) = self.samples.back() else {
            return (0.0, 0.0);
        };
        let Some(&(oldest_at, oldest_rx, oldest_lost)) = self.samples.front() else {
            return (0.0, 0.0);
        };
        let received = newest_rx.saturating_sub(oldest_rx);
        let lost = newest_lost.saturating_sub(oldest_lost);
        let total = received + lost;
        let loss_ratio = if total == 0 {
            0.0
        } else {
            lost as f64 / total as f64
        };
        // A amostra mais próxima de `RATE_SPAN` atrás (ou a mais antiga, no começo).
        let reference = self
            .samples
            .iter()
            .rev()
            .find(|(at, ..)| now.saturating_duration_since(*at) >= RATE_SPAN)
            .copied()
            .unwrap_or((oldest_at, oldest_rx, oldest_lost));
        let span = now.saturating_duration_since(reference.0).as_secs_f64();
        let rate = if span > 0.0 {
            newest_rx.saturating_sub(reference.1) as f64 / span
        } else {
            0.0
        };
        (rate, loss_ratio)
    }
}

struct State {
    histories: HashMap<u32, History>,
    last_advance: Instant,
}

/// Lê as métricas das sessões vivas do [`ControlServer`].
pub struct MetricsCollector {
    control: Arc<ControlServer>,
    thresholds: QualityThresholds,
    state: Mutex<State>,
}

impl MetricsCollector {
    /// Cria o coletor.
    #[must_use]
    pub fn new(control: Arc<ControlServer>, thresholds: QualityThresholds) -> Self {
        Self {
            control,
            thresholds,
            state: Mutex::new(State {
                histories: HashMap::new(),
                last_advance: Instant::now(),
            }),
        }
    }

    /// Tira uma foto agora. A cada segundo, fecha o segundo atual das janelas de 60 s.
    pub fn sample(&self, now: Instant) -> MetricsSnapshot {
        let mut live = self.control.live_sessions();
        live.sort_by_key(|s| s.slot.index());
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);

        let advance = now.saturating_duration_since(state.last_advance) >= Duration::from_secs(1);
        if advance {
            state.last_advance = now;
        }
        state
            .histories
            .retain(|id, _| live.iter().any(|s| s.session_id == *id));

        let slots = live
            .iter()
            .map(|session| {
                let metrics = &session.activity.metrics;
                if advance {
                    metrics.advance_window();
                }
                let snap = metrics.snapshot();
                let history = state.histories.entry(session.session_id).or_default();
                history.push(now, snap.received, snap.lost);
                let (rate_hz, loss_ratio) = history.rates(now);
                let quality = (snap.received > 0).then(|| {
                    let p95 = (snap.rtt_last.is_some()).then_some(snap.rtt_p[1]);
                    self.thresholds.classify(p95, snap.jitter, loss_ratio)
                });
                SlotMetrics {
                    slot: session.slot,
                    session_id: session.session_id,
                    rtt_last: snap.rtt_last,
                    rtt_p: snap.rtt_p,
                    jitter: snap.jitter,
                    loss_ratio,
                    out_of_order: snap.out_of_order,
                    rate_hz,
                    processing_p: snap.processing_p,
                    battery: snap.battery,
                    quality,
                }
            })
            .collect();
        MetricsSnapshot { slots }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: fn(u64) -> Duration = Duration::from_millis;

    #[test]
    fn classification_follows_the_worst_measure() {
        let t = QualityThresholds::default();
        assert_eq!(t.classify(Some(MS(8)), MS(1), 0.0), Quality::Excellent);
        assert_eq!(t.classify(Some(MS(30)), MS(1), 0.0), Quality::Good);
        assert_eq!(t.classify(Some(MS(8)), MS(12), 0.0), Quality::Unstable);
        assert_eq!(t.classify(Some(MS(8)), MS(1), 0.03), Quality::Unstable);
        assert_eq!(t.classify(Some(MS(120)), MS(1), 0.0), Quality::Poor);
        assert_eq!(t.classify(None, MS(1), 0.2), Quality::Poor);
    }

    #[test]
    fn without_rtt_only_jitter_and_loss_count() {
        let t = QualityThresholds::default();
        assert_eq!(t.classify(None, MS(1), 0.0), Quality::Excellent);
    }

    #[test]
    fn history_computes_rate_and_loss() {
        let mut h = History::default();
        let t0 = Instant::now();
        // 250 pacotes/s com 2 em cada 100 perdidos, durante 3 s.
        for i in 0..=12u64 {
            h.push(t0 + MS(250 * i), 62 * i, i);
        }
        let (rate, loss) = h.rates(t0 + MS(3000));
        assert!((rate - 248.0).abs() < 5.0, "taxa {rate}");
        assert!((loss - 12.0 / (12.0 + 744.0)).abs() < 1e-9, "perda {loss}");
    }

    #[test]
    fn old_samples_leave_the_history() {
        let mut h = History::default();
        let t0 = Instant::now();
        h.push(t0, 0, 0);
        h.push(t0 + Duration::from_secs(30), 100, 0);
        assert_eq!(h.samples.len(), 1);
    }
}
