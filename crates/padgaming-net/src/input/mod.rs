//! Loop UDP de input (hot path): recebe o input dos celulares e o entrega ao driver.
//!
//! - [`InputEngine`]: o núcleo sem socket (um datagrama → `XReport` no driver), sem alocação.
//! - [`InputServer`]: a thread `input-rx` com o socket UDP, sessões por comando e parada limpa.
//! - [`InputStats`]: contadores e histograma do tempo de processamento.

mod engine;
mod platform;
mod server;
mod stats;

pub use engine::{Handled, InputEngine, MAX_SESSIONS, SessionActivity, SessionEntry, SessionSpec};
pub use platform::RealtimeTuning;
pub use server::{DEFAULT_PORT, InputConfig, InputServer};
#[cfg(debug_assertions)]
pub use server::{DEV_KEY, DEV_SESSION_ID};
pub use stats::{InputStats, LatencyHistogram, StatsSnapshot};
