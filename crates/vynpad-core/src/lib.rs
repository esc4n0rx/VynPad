//! Orquestração do server: sessões, slots e métricas.

/// Versão do núcleo do `VynPad`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod clock;
pub mod slots;
pub mod xinput;

pub use clock::{Clock, SystemClock};
pub use slots::{SlotConfig, SlotError, SlotManager, SlotSnapshot, SlotView, SlotsSnapshot};
pub use xinput::{XInputOccupant, classify as classify_xinput};
