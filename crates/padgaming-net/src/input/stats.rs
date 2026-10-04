//! Contadores e histograma do loop de input, todos atômicos (sem lock no hot path).

// Contas de histograma: os índices e contagens ficam sempre dentro da faixa dos tipos, e a
// perda de precisão em `f64` é irrelevante para um percentil.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

/// Resolução exata até 16 ns; depois disso, 4 sub-faixas por potência de 2 (erro ≤ 25%).
const EXACT: u64 = 16;
/// Maior potência de 2 distinta (2^40 ns ≈ 18 min); acima disso, tudo cai na última faixa.
const MAX_MSB: u32 = 40;
const SUB_BUCKETS: u64 = 4;
const BUCKETS: usize = (EXACT + (MAX_MSB as u64 - 3) * SUB_BUCKETS) as usize;

/// Histograma de durações em nanossegundos, com escrita concorrente sem lock.
#[derive(Debug)]
pub struct LatencyHistogram {
    buckets: [AtomicU64; BUCKETS],
    count: AtomicU64,
    max_ns: AtomicU64,
}

impl Default for LatencyHistogram {
    fn default() -> Self {
        Self {
            buckets: std::array::from_fn(|_| AtomicU64::new(0)),
            count: AtomicU64::new(0),
            max_ns: AtomicU64::new(0),
        }
    }
}

fn bucket_of(ns: u64) -> usize {
    if ns < EXACT {
        return ns as usize;
    }
    let msb = ns.ilog2();
    if msb > MAX_MSB {
        return BUCKETS - 1;
    }
    let sub = (ns >> (msb - 2)) & (SUB_BUCKETS - 1);
    (EXACT + u64::from(msb - 4) * SUB_BUCKETS + sub) as usize
}

/// Limite superior (exclusivo) da faixa, em ns.
fn upper_bound(bucket: usize) -> u64 {
    let b = bucket as u64;
    if b < EXACT {
        return b + 1;
    }
    let idx = b - EXACT;
    let msb = idx / SUB_BUCKETS + 4;
    let sub = idx % SUB_BUCKETS;
    let width = 1u64 << (msb - 2);
    (1u64 << msb) + (sub + 1) * width
}

impl LatencyHistogram {
    /// Registra uma duração.
    pub fn record(&self, elapsed: Duration) {
        let ns = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);
        self.buckets[bucket_of(ns)].fetch_add(1, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
        self.max_ns.fetch_max(ns, Ordering::Relaxed);
    }

    /// Quantidade de amostras.
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Maior duração registrada.
    pub fn max(&self) -> Duration {
        Duration::from_nanos(self.max_ns.load(Ordering::Relaxed))
    }

    fn clear(&self) {
        for bucket in &self.buckets {
            bucket.store(0, Ordering::Relaxed);
        }
        self.count.store(0, Ordering::Relaxed);
        self.max_ns.store(0, Ordering::Relaxed);
    }

    /// Limite superior da faixa que contém o percentil `p` (0–100); `ZERO` sem amostras.
    /// O valor real é no máximo este, e no mínimo 80% dele.
    pub fn percentile(&self, p: f64) -> Duration {
        let total = self.count();
        if total == 0 {
            return Duration::ZERO;
        }
        let rank = ((p / 100.0) * total as f64).ceil().max(1.0) as u64;
        let mut seen = 0;
        for (i, bucket) in self.buckets.iter().enumerate() {
            seen += bucket.load(Ordering::Relaxed);
            if seen >= rank {
                return Duration::from_nanos(upper_bound(i));
            }
        }
        self.max()
    }
}

/// Quantos segundos a janela deslizante guarda.
pub const WINDOW_SECS: usize = 60;

/// Histograma dos últimos [`WINDOW_SECS`] segundos: um [`LatencyHistogram`] por segundo.
///
/// Quem mede grava sem lock em [`Self::record`]; o coletor chama [`Self::advance`] uma vez por
/// segundo (fora do hot path), o que descarta o segundo mais antigo.
#[derive(Debug)]
pub struct RollingHistogram {
    slices: [LatencyHistogram; WINDOW_SECS],
    cursor: AtomicUsize,
}

impl Default for RollingHistogram {
    fn default() -> Self {
        Self {
            slices: std::array::from_fn(|_| LatencyHistogram::default()),
            cursor: AtomicUsize::new(0),
        }
    }
}

impl RollingHistogram {
    /// Registra uma duração no segundo atual.
    pub fn record(&self, elapsed: Duration) {
        self.slices[self.cursor.load(Ordering::Relaxed)].record(elapsed);
    }

    /// Passa para o próximo segundo e esquece o mais antigo.
    pub fn advance(&self) {
        let next = (self.cursor.load(Ordering::Relaxed) + 1) % WINDOW_SECS;
        self.slices[next].clear();
        self.cursor.store(next, Ordering::Relaxed);
    }

    /// Amostras na janela.
    pub fn count(&self) -> u64 {
        self.slices.iter().map(LatencyHistogram::count).sum()
    }

    /// Percentil `p` (0–100) de toda a janela; `ZERO` sem amostras. Mesma precisão do
    /// [`LatencyHistogram::percentile`].
    pub fn percentile(&self, p: f64) -> Duration {
        let total = self.count();
        if total == 0 {
            return Duration::ZERO;
        }
        let rank = ((p / 100.0) * total as f64).ceil().max(1.0) as u64;
        let mut seen = 0;
        for i in 0..BUCKETS {
            seen += self
                .slices
                .iter()
                .map(|s| s.buckets[i].load(Ordering::Relaxed))
                .sum::<u64>();
            if seen >= rank {
                return Duration::from_nanos(upper_bound(i));
            }
        }
        self.slices
            .iter()
            .map(LatencyHistogram::max)
            .max()
            .unwrap_or(Duration::ZERO)
    }
}

/// Contadores do loop de input.
#[derive(Debug, Default)]
pub struct InputStats {
    /// Datagramas recebidos.
    pub received: AtomicU64,
    /// Inputs autênticos entregues ao pipeline.
    pub inputs: AtomicU64,
    /// Pings respondidos.
    pub pings: AtomicU64,
    /// Descartados: cabeçalho, tamanho, tipo ou versão inválidos.
    pub dropped_malformed: AtomicU64,
    /// Descartados: `session_id` desconhecido.
    pub dropped_unknown_session: AtomicU64,
    /// Descartados: tag de autenticação inválida (adulterado, ou chave errada).
    pub dropped_auth: AtomicU64,
    /// Descartados: `seq` repetido ou antigo.
    pub dropped_replay: AtomicU64,
    /// Inputs válidos que o `SlotManager` recusou (driver indisponível, sessão suspensa...).
    pub submit_errors: AtomicU64,
    /// Pacotes `Rumble` enviados (inclui as retransmissões).
    pub rumble_sent: AtomicU64,
    /// Tempo de processamento por datagrama (da chegada até o `update` do driver).
    pub processing: LatencyHistogram,
}

/// Foto dos contadores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatsSnapshot {
    /// Datagramas recebidos.
    pub received: u64,
    /// Inputs entregues.
    pub inputs: u64,
    /// Pings respondidos.
    pub pings: u64,
    /// Descartados por formato.
    pub dropped_malformed: u64,
    /// Descartados por sessão desconhecida.
    pub dropped_unknown_session: u64,
    /// Descartados por autenticação.
    pub dropped_auth: u64,
    /// Descartados por replay.
    pub dropped_replay: u64,
    /// Recusados pelo `SlotManager`.
    pub submit_errors: u64,
    /// `Rumble` enviados.
    pub rumble_sent: u64,
}

impl StatsSnapshot {
    /// Soma de todos os descartes.
    pub const fn dropped(&self) -> u64 {
        self.dropped_malformed
            + self.dropped_unknown_session
            + self.dropped_auth
            + self.dropped_replay
    }
}

impl InputStats {
    /// Lê todos os contadores.
    pub fn snapshot(&self) -> StatsSnapshot {
        let get = |c: &AtomicU64| c.load(Ordering::Relaxed);
        StatsSnapshot {
            received: get(&self.received),
            inputs: get(&self.inputs),
            pings: get(&self.pings),
            dropped_malformed: get(&self.dropped_malformed),
            dropped_unknown_session: get(&self.dropped_unknown_session),
            dropped_auth: get(&self.dropped_auth),
            dropped_replay: get(&self.dropped_replay),
            submit_errors: get(&self.submit_errors),
            rumble_sent: get(&self.rumble_sent),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_are_monotonic_and_cover_their_values() {
        let mut last = 0;
        for ns in [
            0u64,
            1,
            15,
            16,
            17,
            31,
            32,
            100,
            999,
            1_000,
            65_535,
            1_000_000,
            1u64 << 39,
        ] {
            let b = bucket_of(ns);
            assert!(b >= last, "bucket de {ns} regrediu");
            assert!(upper_bound(b) > ns, "{ns} fora da faixa {b}");
            // Erro relativo do limite superior: no máximo 25% (acima da faixa exata).
            if ns >= 16 {
                assert!(
                    upper_bound(b) as f64 <= ns as f64 * 1.26,
                    "{ns} -> {}",
                    upper_bound(b)
                );
            }
            last = b;
        }
        assert_eq!(bucket_of(u64::MAX), BUCKETS - 1);
        assert!(bucket_of(1u64 << 40) < BUCKETS);
    }

    #[test]
    fn rolling_histogram_forgets_old_seconds() {
        let h = RollingHistogram::default();
        assert_eq!(h.percentile(50.0), Duration::ZERO);
        for _ in 0..100 {
            h.record(Duration::from_millis(40));
        }
        h.advance();
        for _ in 0..100 {
            h.record(Duration::from_millis(2));
        }
        assert_eq!(h.count(), 200);
        // Metade das amostras é de 2 ms e a outra metade de 40 ms.
        assert!(h.percentile(40.0) <= Duration::from_micros(2_600));
        assert!(h.percentile(99.0) >= Duration::from_millis(40));
        // Depois de uma janela inteira, o 40 ms sai.
        for _ in 0..WINDOW_SECS - 1 {
            h.advance();
        }
        assert_eq!(h.count(), 100);
        assert!(h.percentile(99.0) <= Duration::from_micros(2_600));
        h.advance();
        assert_eq!(h.count(), 0);
    }

    #[test]
    fn percentiles_follow_the_distribution() {
        let h = LatencyHistogram::default();
        assert_eq!(h.percentile(99.0), Duration::ZERO);
        for _ in 0..990 {
            h.record(Duration::from_micros(20));
        }
        for _ in 0..10 {
            h.record(Duration::from_micros(800));
        }
        assert_eq!(h.count(), 1000);
        assert_eq!(h.max(), Duration::from_micros(800));
        let p50 = h.percentile(50.0);
        assert!(
            p50 >= Duration::from_micros(20) && p50 <= Duration::from_micros(26),
            "{p50:?}"
        );
        let p99 = h.percentile(99.0);
        assert!(
            p99 <= Duration::from_micros(26),
            "1% lento não deve mover o p99: {p99:?}"
        );
        let p999 = h.percentile(99.9);
        assert!(p999 >= Duration::from_micros(800), "{p999:?}");
    }

    #[test]
    fn snapshot_reads_every_counter() {
        let s = InputStats::default();
        s.received.fetch_add(5, Ordering::Relaxed);
        s.dropped_auth.fetch_add(2, Ordering::Relaxed);
        s.dropped_replay.fetch_add(1, Ordering::Relaxed);
        let snap = s.snapshot();
        assert_eq!(
            (snap.received, snap.dropped_auth, snap.dropped_replay),
            (5, 2, 1)
        );
        assert_eq!(snap.dropped(), 3);
    }
}
