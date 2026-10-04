//! Leitura de durações (`250ms`, `2s`, `1m`), percentuais (`10%`) e um gerador pseudoaleatório
//! pequeno e determinístico (`--seed`), sem depender de outras crates.

use std::time::Duration;

/// Lê uma duração com unidade: `ms`, `s` ou `m` (aceita decimais: `1.5s`).
pub fn parse_duration(text: &str) -> Result<Duration, String> {
    let text = text.trim();
    let (number, factor_ms) = if let Some(n) = text.strip_suffix("ms") {
        (n, 1.0)
    } else if let Some(n) = text.strip_suffix('s') {
        (n, 1_000.0)
    } else if let Some(n) = text.strip_suffix('m') {
        (n, 60_000.0)
    } else {
        return Err(format!("duração `{text}` sem unidade (use ms, s ou m)"));
    };
    let value: f64 = number
        .trim()
        .parse()
        .map_err(|_| format!("duração `{text}` inválida"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!("duração `{text}` inválida"));
    }
    Ok(Duration::from_secs_f64(value * factor_ms / 1_000.0))
}

/// Lê um percentual (`10%`, `0.5%`) e devolve a fração (0 a 1).
pub fn parse_percent(text: &str) -> Result<f64, String> {
    let number = text
        .trim()
        .strip_suffix('%')
        .ok_or_else(|| format!("percentual `{text}` sem `%`"))?;
    let value: f64 = number
        .trim()
        .parse()
        .map_err(|_| format!("percentual `{text}` inválido"))?;
    if !(0.0..=100.0).contains(&value) {
        return Err(format!("percentual `{text}` fora de 0% a 100%"));
    }
    Ok(value / 100.0)
}

/// xorshift64*: rápido, sem alocação e reproduzível com a mesma semente.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// Cria o gerador; semente 0 é trocada por uma constante (o xorshift não aceita zero).
    pub const fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    /// Próximo valor de 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Fração em `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// `true` com probabilidade `p`.
    pub fn chance(&mut self, p: f64) -> bool {
        p > 0.0 && self.unit() < p
    }

    /// Valor entre -1 e 1.
    pub fn signed(&mut self) -> f32 {
        (self.unit() * 2.0 - 1.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_parse_with_units_and_decimals() {
        assert_eq!(parse_duration("250ms").unwrap(), Duration::from_millis(250));
        assert_eq!(parse_duration("2s").unwrap(), Duration::from_secs(2));
        assert_eq!(parse_duration("1.5s").unwrap(), Duration::from_millis(1500));
        assert_eq!(parse_duration("1m").unwrap(), Duration::from_secs(60));
        assert_eq!(parse_duration(" 0ms ").unwrap(), Duration::ZERO);
        for bad in ["10", "abc", "-1s", "s", "1h", "nan s"] {
            assert!(parse_duration(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn percentages_parse_to_fractions() {
        assert!((parse_percent("10%").unwrap() - 0.1).abs() < 1e-12);
        assert_eq!(parse_percent("0%").unwrap(), 0.0);
        assert_eq!(parse_percent("100%").unwrap(), 1.0);
        for bad in ["10", "101%", "-1%", "x%"] {
            assert!(parse_percent(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn rng_is_reproducible_and_roughly_uniform() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        assert_eq!(
            (0..10).map(|_| a.next_u64()).collect::<Vec<_>>(),
            (0..10).map(|_| b.next_u64()).collect::<Vec<_>>()
        );
        assert_ne!(Rng::new(0).next_u64(), 0);

        let mut rng = Rng::new(7);
        let hits = (0..10_000).filter(|_| rng.chance(0.3)).count();
        assert!((2_700..3_300).contains(&hits), "{hits}");
        assert!((0..1000).all(|_| (-1.0..=1.0).contains(&rng.signed())));
        assert!(!Rng::new(1).chance(0.0));
        assert!(Rng::new(1).chance(1.0));
    }
}
