//! Relógio injetável: os testes avançam o tempo sem `sleep`.

use std::time::Instant;

/// Fonte de tempo monotônico.
pub trait Clock: Send + Sync {
    /// Instante atual.
    fn now(&self) -> Instant;
}

/// Relógio do sistema.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}
