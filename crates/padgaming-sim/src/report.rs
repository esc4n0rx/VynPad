//! Relatório do `bench`: texto para ler e JSON para máquinas.

use std::fmt::Write as _;
use std::time::Duration;

use serde::Serialize;

use crate::client::ClientReport;

/// Percentis de uma série de tempos, em µs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub struct Percentiles {
    /// Amostras.
    pub count: usize,
    /// Mediana.
    pub p50: u32,
    /// Percentil 95.
    pub p95: u32,
    /// Percentil 99.
    pub p99: u32,
    /// Maior valor.
    pub max: u32,
}

impl Percentiles {
    /// Calcula os percentis (ordena uma cópia).
    pub fn of(samples: &[u32]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }
        let mut sorted = samples.to_vec();
        sorted.sort_unstable();
        // Método do posto mais próximo: o menor valor com pelo menos p% das amostras abaixo.
        let at = |p: f64| {
            let rank = ((p / 100.0) * sorted.len() as f64).ceil().max(1.0) as usize;
            sorted[rank.min(sorted.len()) - 1]
        };
        Self {
            count: sorted.len(),
            p50: at(50.0),
            p95: at(95.0),
            p99: at(99.0),
            max: *sorted.last().expect("não vazio"),
        }
    }
}

/// Resumo de um client (ou do conjunto).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Summary {
    /// Pacotes de input gerados.
    pub inputs_sent: u64,
    /// Perdidos pela falha simulada.
    pub lost_by_fault: u64,
    /// Pings enviados.
    pub pings_sent: u64,
    /// Pongs recebidos.
    pub pongs: u64,
    /// Pings sem resposta, em %.
    pub ping_loss_percent: f64,
    /// RTT em µs.
    pub rtt_us: Percentiles,
    /// Tempo de processamento no server (`server_proc_us`), em µs.
    pub server_proc_us: Percentiles,
    /// Pacotes de vibração recebidos.
    pub rumbles: u64,
}

impl Summary {
    /// Resume um client.
    pub fn of(report: &ClientReport) -> Self {
        Self {
            inputs_sent: report.inputs_sent,
            lost_by_fault: report.lost_by_fault,
            pings_sent: report.pings_sent,
            pongs: report.pongs,
            ping_loss_percent: if report.pings_sent == 0 {
                0.0
            } else {
                100.0 * (1.0 - report.pongs as f64 / report.pings_sent as f64).max(0.0)
            },
            rtt_us: Percentiles::of(&report.rtt_us),
            server_proc_us: Percentiles::of(&report.server_proc_us),
            rumbles: report.rumbles,
        }
    }
}

/// Parâmetros da execução, repetidos no relatório para ele ser autoexplicativo.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BenchParams {
    /// Endereço do server.
    pub server: String,
    /// Clients simultâneos.
    pub clients: usize,
    /// Pacotes de input por segundo, por client.
    pub rate_hz: u32,
    /// Duração pedida, em segundos.
    pub duration_s: f64,
    /// Perda simulada, em %.
    pub loss_percent: f64,
    /// Atraso simulado, em ms.
    pub delay_ms: f64,
    /// Jitter simulado, em ms.
    pub jitter_ms: f64,
    /// Reordenação simulada.
    pub reorder: bool,
}

/// Relatório completo do `bench`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BenchReport {
    /// Como a carga foi gerada.
    pub params: BenchParams,
    /// Duração real, em segundos.
    pub elapsed_s: f64,
    /// Um resumo por client.
    pub clients: Vec<Summary>,
    /// Todos os clients juntos (percentis sobre todas as amostras).
    pub total: Summary,
}

impl BenchReport {
    /// Monta o relatório a partir dos resultados de cada client.
    pub fn new(params: BenchParams, reports: &[ClientReport], elapsed: Duration) -> Self {
        let mut all = ClientReport::default();
        for r in reports {
            all.inputs_sent += r.inputs_sent;
            all.lost_by_fault += r.lost_by_fault;
            all.pings_sent += r.pings_sent;
            all.pongs += r.pongs;
            all.rumbles += r.rumbles;
            all.rtt_us.extend_from_slice(&r.rtt_us);
            all.server_proc_us.extend_from_slice(&r.server_proc_us);
        }
        Self {
            params,
            elapsed_s: elapsed.as_secs_f64(),
            clients: reports.iter().map(Summary::of).collect(),
            total: Summary::of(&all),
        }
    }

    /// JSON legível.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("relatório sempre serializa")
    }

    /// Texto para o terminal.
    pub fn to_text(&self) -> String {
        let ms = |us: u32| f64::from(us) / 1000.0;
        let p = &self.params;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "bench: {} clients x {} Hz por {:.0}s contra {} (real: {:.1}s)",
            p.clients, p.rate_hz, p.duration_s, p.server, self.elapsed_s
        );
        if p.loss_percent > 0.0 || p.delay_ms > 0.0 || p.jitter_ms > 0.0 || p.reorder {
            let _ = writeln!(
                out,
                "falhas: perda {}%  atraso {} ms  jitter {} ms  reorder {}",
                p.loss_percent, p.delay_ms, p.jitter_ms, p.reorder
            );
        }
        let _ = writeln!(
            out,
            "\n{:<8} {:>9} {:>7} {:>7} {:>9} {:>9} {:>9} {:>9}",
            "client", "inputs", "pings", "pongs", "rtt p50", "rtt p95", "rtt p99", "rtt max"
        );
        let row = |out: &mut String, name: String, s: &Summary| {
            let _ = writeln!(
                out,
                "{:<8} {:>9} {:>7} {:>7} {:>7.2}ms {:>7.2}ms {:>7.2}ms {:>7.2}ms",
                name,
                s.inputs_sent,
                s.pings_sent,
                s.pongs,
                ms(s.rtt_us.p50),
                ms(s.rtt_us.p95),
                ms(s.rtt_us.p99),
                ms(s.rtt_us.max)
            );
        };
        for (i, c) in self.clients.iter().enumerate() {
            row(&mut out, format!("#{}", i + 1), c);
        }
        row(&mut out, "total".to_owned(), &self.total);
        let t = &self.total;
        let _ = writeln!(
            out,
            "\nperda simulada: {} pacotes | pings sem resposta: {:.2}% | processamento no server (p99): {:.3} ms",
            t.lost_by_fault,
            t.ping_loss_percent,
            ms(t.server_proc_us.p99)
        );
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_use_nearest_rank() {
        let samples: Vec<u32> = (1..=100).collect();
        let p = Percentiles::of(&samples);
        assert_eq!(
            (p.count, p.p50, p.p95, p.p99, p.max),
            (100, 50, 95, 99, 100)
        );
        assert_eq!(Percentiles::of(&[]), Percentiles::default());
        let one = Percentiles::of(&[7]);
        assert_eq!((one.p50, one.p99, one.max), (7, 7, 7));
        // Ordem de entrada não importa.
        assert_eq!(Percentiles::of(&[5, 1, 3]).p50, 3);
    }

    fn client(sent: u64, answered: u64, rtts: &[u32]) -> ClientReport {
        ClientReport {
            inputs_sent: 1000,
            lost_by_fault: 3,
            pings_sent: sent,
            pongs: answered,
            rtt_us: rtts.to_vec(),
            server_proc_us: vec![10; rtts.len()],
            ..ClientReport::default()
        }
    }

    fn params() -> BenchParams {
        BenchParams {
            server: "127.0.0.1:47801".into(),
            clients: 2,
            rate_hz: 250,
            duration_s: 4.0,
            loss_percent: 0.0,
            delay_ms: 0.0,
            jitter_ms: 0.0,
            reorder: false,
        }
    }

    #[test]
    fn the_total_merges_every_client_and_reports_ping_loss() {
        let a = client(20, 20, &[1000, 2000]);
        let b = client(20, 10, &[3000, 4000]);
        let report = BenchReport::new(params(), &[a, b], Duration::from_secs(4));
        assert_eq!(report.clients.len(), 2);
        assert_eq!(report.total.inputs_sent, 2000);
        assert_eq!(report.total.pongs, 30);
        assert_eq!(report.total.rtt_us.count, 4);
        assert_eq!(report.total.rtt_us.max, 4000);
        assert!((report.total.ping_loss_percent - 25.0).abs() < 1e-9);
        assert_eq!(report.clients[0].ping_loss_percent, 0.0);
    }

    #[test]
    fn json_and_text_carry_the_same_numbers() {
        let report = BenchReport::new(
            params(),
            &[client(20, 20, &[1500, 2500])],
            Duration::from_secs(4),
        );
        let json: serde_json::Value = serde_json::from_str(&report.to_json()).unwrap();
        assert_eq!(json["params"]["clients"], 2);
        assert_eq!(json["total"]["rtt_us"]["max"], 2500);
        assert_eq!(json["clients"][0]["pongs"], 20);

        let text = report.to_text();
        assert!(text.contains("2 clients x 250 Hz"));
        assert!(text.contains("2.50ms"), "{text}");
        assert!(text.contains("total"));
        assert!(
            !text.contains("falhas:"),
            "sem falhas simuladas, a linha não aparece"
        );

        let mut faulty = params();
        faulty.loss_percent = 10.0;
        let text =
            BenchReport::new(faulty, &[client(20, 20, &[1])], Duration::from_secs(1)).to_text();
        assert!(text.contains("falhas: perda 10%"));
    }
}
