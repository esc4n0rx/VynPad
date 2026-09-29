//! Filtro 1-Euro (Casiez et al., CHI 2012): suaviza o tremor com pouco atraso ao mover rápido.

use core::f32::consts::TAU;

/// Frequência de corte da derivada, em Hz (valor recomendado pelo artigo).
const DERIVATIVE_CUTOFF: f32 = 1.0;
/// Menor `dt` aceito, para não dividir por zero com timestamps repetidos.
const MIN_DT: f32 = 1e-4;

/// Estado de um filtro 1-Euro de um eixo.
#[derive(Debug, Clone, Copy, Default)]
pub struct OneEuro {
    /// `(valor filtrado, derivada filtrada)` da última amostra.
    last: Option<(f32, f32)>,
}

fn alpha(cutoff: f32, dt: f32) -> f32 {
    let tau = 1.0 / (TAU * cutoff);
    1.0 / (1.0 + tau / dt)
}

impl OneEuro {
    /// Esquece o histórico: a próxima amostra passa sem filtro.
    pub fn reset(&mut self) {
        self.last = None;
    }

    /// Filtra uma amostra `x` obtida `dt` segundos depois da anterior.
    pub fn filter(&mut self, x: f32, dt: f32, min_cutoff: f32, beta: f32) -> f32 {
        let dt = dt.max(MIN_DT);
        let Some((last_value, last_slope)) = self.last else {
            self.last = Some((x, 0.0));
            return x;
        };
        let dx = (x - last_value) / dt;
        let dx_hat = last_slope + alpha(DERIVATIVE_CUTOFF, dt) * (dx - last_slope);
        let cutoff = min_cutoff + beta * dx_hat.abs();
        let x_hat = last_value + alpha(cutoff, dt) * (x - last_value);
        self.last = Some((x_hat, dx_hat));
        x_hat
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // comparações exatas são intencionais nestes testes
mod tests {
    use super::*;

    #[test]
    fn first_sample_passes_through_and_reset_forgets() {
        let mut f = OneEuro::default();
        assert_eq!(f.filter(3.0, 0.004, 1.0, 0.0), 3.0);
        f.reset();
        assert_eq!(f.filter(-7.0, 0.004, 1.0, 0.0), -7.0);
    }

    #[test]
    fn constant_input_stays_constant() {
        let mut f = OneEuro::default();
        for _ in 0..100 {
            assert!((f.filter(1.0, 0.004, 1.0, 0.05) - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn smooths_jitter() {
        let mut f = OneEuro::default();
        let mut out = Vec::new();
        for i in 0..200 {
            let noisy = if i % 2 == 0 { 0.1 } else { -0.1 };
            out.push(f.filter(noisy, 0.004, 1.0, 0.0));
        }
        let tail = &out[100..];
        let swing = tail.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
        assert!(
            swing < 0.02,
            "o filtro deveria amortecer o tremor, sobrou {swing}"
        );
    }

    #[test]
    fn higher_beta_follows_fast_movement_with_less_lag() {
        let run = |beta: f32| {
            let mut f = OneEuro::default();
            f.filter(0.0, 0.004, 1.0, beta);
            let mut y = 0.0;
            for _ in 0..25 {
                y = f.filter(1.0, 0.004, 1.0, beta);
            }
            y
        };
        assert!(run(5.0) > run(0.0));
    }

    #[test]
    fn zero_or_negative_dt_does_not_produce_nan() {
        let mut f = OneEuro::default();
        f.filter(0.0, 0.004, 1.0, 0.05);
        for dt in [0.0, -1.0, f32::MIN_POSITIVE] {
            assert!(f.filter(1.0, dt, 1.0, 0.05).is_finite());
        }
    }
}
