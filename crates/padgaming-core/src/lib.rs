//! Orquestração do server: sessões, slots e métricas.

/// Versão do núcleo do `PadGaming`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod clock;
pub mod slots;

pub use clock::{Clock, SystemClock};
pub use slots::{SlotConfig, SlotError, SlotManager, SlotSnapshot, SlotView, SlotsSnapshot};
