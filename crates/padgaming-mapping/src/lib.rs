//! Pipeline puro de mapeamento: deadzones, curvas, gyro e remapeamento de botões.
//!
//! ```
//! use padgaming_mapping::{CompiledProfile, MapperState, RawInput, apply};
//!
//! let profile = CompiledProfile::standard(); // identidade + deadzone de 8%
//! let mut state = MapperState::new();
//! let raw = RawInput { lx: 32767, ..RawInput::default() };
//! let report = apply(&profile, &raw, &mut state, 0.004);
//! assert_eq!(report.lx, 32767);
//! ```
//!
//! - [`Profile`]: o que o usuário edita (JSON versionado, validado).
//! - [`CompiledProfile`]: o perfil com tabelas pré-calculadas, pronto para o hot path.
//! - [`apply`]: função pura `perfil + entrada + estado + dt → XReport`, sem alocação.
//! - [`ProfileCell`] / [`Mapper`]: troca de perfil em tempo real, sem travar o input.

// As conversões entre `f32` e inteiros de 8/16 bits são a essência do pipeline e ficam sempre
// dentro da faixa (valores são limitados antes; `as` satura e NaN vira 0).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub mod compiled;
pub mod filter;
pub mod input;
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub mod pipeline;
pub mod profile;

pub use compiled::{CompiledProfile, GYRO_FULL_SCALE_RAD_S};
pub use input::{Button, Gyro, RawInput};
pub use pipeline::{Mapper, MapperState, ProfileCell, apply};
pub use profile::{
    ButtonTarget, ButtonsConfig, Curve, DeadzoneShape, GyroAxis, GyroConfig, GyroMode, Profile,
    ProfileError, RumbleConfig, SCHEMA_VERSION, Smoothing, StickConfig, StickSide, SticksConfig,
    TriggerConfig, TriggersConfig, Turbo,
};
