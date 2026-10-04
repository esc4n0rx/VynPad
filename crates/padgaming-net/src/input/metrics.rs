//! Métricas de uma sessão: perda, jitter, RTT, bateria e tempo de processamento.
//!
//! Quem escreve é o loop de input (um só thread) e o servidor de controle (o RTT); quem lê é o
//! coletor da camada de composição, 4 vezes por segundo. Tudo é atômico: nenhum lock no hot path,
//! e o custo por pacote são poucas operações `Relaxed` (medido em `per_packet_cost_is_negligible`).

use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

use super::stats::RollingHistogram;

/// Intervalos maiores que isto (pausa, reconexão) não entram no jitter.
const MAX_JITTER_INTERVAL: Duration = Duration::from_millis(500);
/// Valor de `rtt_last_us` quando ainda não houve medição.
const NO_RTT: u32 = u32::MAX;
/// Suavização do intervalo médio e do jitter (1/16, como no RFC 3550).
const SMOOTHING_SHIFT: u32 = 4;

/// Métricas de uma sessão de input.
#[derive(Debug)]
pub struct SessionMetrics {
    received: AtomicU64,
    lost: AtomicU64,
    out_of_order: AtomicU64,
    last_seq: AtomicU32,
    seq_seen: AtomicU8,
    /// Média móvel do intervalo entre inputs, em µs.
    mean_us: AtomicU32,
    /// Desvio médio do intervalo em relação à média, em µs.
    jitter_us: AtomicU32,
    battery: AtomicU8,
    rtt_last_us: AtomicU32,
    processing: RollingHistogram,
    rtt: RollingHistogram,
}

impl Default for SessionMetrics {
    fn default() -> Self {
        Self {
            received: AtomicU64::new(0),
            lost: AtomicU64::new(0),
            out_of_order: AtomicU64::new(0),
            last_seq: AtomicU32::new(0),
            seq_seen: AtomicU8::new(0),
            mean_us: AtomicU32::new(0),
            jitter_us: AtomicU32::new(0),
            battery: AtomicU8::new(u8::MAX),
            rtt_last_us: AtomicU32::new(NO_RTT),
            processing: RollingHistogram::default(),
            rtt: RollingHistogram::default(),
        }
    }
}

/// Foto das métricas de uma sessão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetricsSnapshot {
    /// Pacotes autênticos recebidos desde o início da sessão.
    pub received: u64,
    /// Pacotes que o `seq` mostra que faltaram.
    pub lost: u64,
    /// Pacotes recusados por `seq` repetido ou antigo (fora de ordem ou duplicado).
    pub out_of_order: u64,
    /// Jitter do intervalo entre inputs.
    pub jitter: Duration,
    /// Bateria do celular (0–100), se informada.
    pub battery: Option<u8>,
    /// Último RTT medido.
    pub rtt_last: Option<Duration>,
    /// RTT p50/p95/p99 dos últimos 60 s.
    pub rtt_p: [Duration; 3],
    /// Tempo de processamento p50/p95/p99 dos últimos 60 s.
    pub processing_p: [Duration; 3],
}

impl SessionMetrics {
    /// Um pacote autêntico chegou, com o `seq` dele.
    #[inline]
    pub fn on_packet(&self, seq: u32) {
        self.received.fetch_add(1, Ordering::Relaxed);
        if self.seq_seen.swap(1, Ordering::Relaxed) == 1 {
            let last = self.last_seq.load(Ordering::Relaxed);
            if seq > last.saturating_add(1) {
                self.lost
                    .fetch_add(u64::from(seq - last - 1), Ordering::Relaxed);
            }
        }
        self.last_seq.store(seq, Ordering::Relaxed);
    }

    /// Um pacote foi recusado por `seq` repetido ou antigo.
    #[inline]
    pub fn on_stale(&self) {
        self.out_of_order.fetch_add(1, Ordering::Relaxed);
    }

    /// Passou `interval` desde o input anterior.
    #[inline]
    pub fn on_interval(&self, interval: Duration) {
        if interval > MAX_JITTER_INTERVAL {
            return;
        }
        let us = i64::from(u32::try_from(interval.as_micros()).unwrap_or(u32::MAX));
        let mean = i64::from(self.mean_us.load(Ordering::Relaxed));
        let mean = if mean == 0 {
            us
        } else {
            mean + ((us - mean) >> SMOOTHING_SHIFT)
        };
        let jitter = i64::from(self.jitter_us.load(Ordering::Relaxed));
        let jitter = jitter + (((us - mean).abs() - jitter) >> SMOOTHING_SHIFT);
        self.mean_us
            .store(u32::try_from(mean).unwrap_or(u32::MAX), Ordering::Relaxed);
        self.jitter_us
            .store(u32::try_from(jitter).unwrap_or(u32::MAX), Ordering::Relaxed);
    }

    /// Bateria bruta do `InputState` (`255` = desconhecida).
    #[inline]
    pub fn set_battery(&self, raw: u8) {
        self.battery.store(raw, Ordering::Relaxed);
    }

    /// Tempo entre a chegada do datagrama e o `update` do driver.
    #[inline]
    pub fn record_processing(&self, elapsed: Duration) {
        self.processing.record(elapsed);
    }

    /// RTT medido (pelo canal de controle).
    pub fn record_rtt(&self, rtt: Duration) {
        self.rtt_last_us.store(
            u32::try_from(rtt.as_micros()).unwrap_or(u32::MAX - 1),
            Ordering::Relaxed,
        );
        self.rtt.record(rtt);
    }

    /// Fecha o segundo atual nas janelas (uma vez por segundo, pelo coletor).
    pub fn advance_window(&self) {
        self.processing.advance();
        self.rtt.advance();
    }

    /// Lê tudo de uma vez.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let battery = self.battery.load(Ordering::Relaxed);
        let rtt_last = self.rtt_last_us.load(Ordering::Relaxed);
        let percentiles = |h: &RollingHistogram| [50.0, 95.0, 99.0].map(|p| h.percentile(p));
        MetricsSnapshot {
            received: self.received.load(Ordering::Relaxed),
            lost: self.lost.load(Ordering::Relaxed),
            out_of_order: self.out_of_order.load(Ordering::Relaxed),
            jitter: Duration::from_micros(u64::from(self.jitter_us.load(Ordering::Relaxed))),
            battery: (battery <= 100).then_some(battery),
            rtt_last: (rtt_last != NO_RTT).then(|| Duration::from_micros(u64::from(rtt_last))),
            rtt_p: percentiles(&self.rtt),
            processing_p: percentiles(&self.processing),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[test]
    fn gaps_in_seq_count_as_lost_and_stale_as_out_of_order() {
        let m = SessionMetrics::default();
        for seq in [1, 2, 3, 6, 7] {
            m.on_packet(seq);
        }
        m.on_stale();
        let s = m.snapshot();
        assert_eq!(s.received, 5);
        assert_eq!(s.lost, 2); // 4 e 5
        assert_eq!(s.out_of_order, 1);
    }

    #[test]
    fn jitter_is_zero_for_a_steady_stream_and_grows_with_variation() {
        let steady = SessionMetrics::default();
        for _ in 0..200 {
            steady.on_interval(Duration::from_millis(4));
        }
        assert!(steady.snapshot().jitter <= Duration::from_micros(50));

        let noisy = SessionMetrics::default();
        for i in 0..200 {
            noisy.on_interval(Duration::from_millis(if i % 2 == 0 { 2 } else { 8 }));
        }
        let jitter = noisy.snapshot().jitter;
        assert!(
            jitter >= Duration::from_millis(2) && jitter <= Duration::from_millis(4),
            "{jitter:?}"
        );
    }

    #[test]
    fn long_pauses_do_not_count_as_jitter() {
        let m = SessionMetrics::default();
        for _ in 0..50 {
            m.on_interval(Duration::from_millis(4));
        }
        m.on_interval(Duration::from_secs(3));
        assert!(m.snapshot().jitter <= Duration::from_micros(50));
    }

    #[test]
    fn battery_and_rtt_start_unknown() {
        let m = SessionMetrics::default();
        let s = m.snapshot();
        assert_eq!(s.battery, None);
        assert_eq!(s.rtt_last, None);
        m.set_battery(87);
        m.record_rtt(Duration::from_micros(21_300));
        let s = m.snapshot();
        assert_eq!(s.battery, Some(87));
        assert_eq!(s.rtt_last, Some(Duration::from_micros(21_300)));
        assert!(s.rtt_p[1] >= Duration::from_micros(21_300));
        m.set_battery(255);
        assert_eq!(m.snapshot().battery, None);
    }

    /// O custo por pacote precisa ficar bem abaixo de 1 µs (T-305). Em debug o limite é folgado;
    /// em release a conta real fica em dezenas de ns.
    #[test]
    fn per_packet_cost_is_negligible() {
        const N: u32 = 200_000;
        let m = SessionMetrics::default();
        let start = Instant::now();
        for seq in 1..=N {
            m.on_packet(seq);
            m.on_interval(Duration::from_micros(4_000 + u64::from(seq % 7)));
            m.set_battery(80);
            m.record_processing(Duration::from_micros(30));
        }
        let per_packet = start.elapsed() / N;
        assert!(
            per_packet < Duration::from_micros(1),
            "{per_packet:?} por pacote"
        );
    }
}
