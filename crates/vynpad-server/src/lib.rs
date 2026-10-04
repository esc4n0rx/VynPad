//! Composição do server: junta o driver, os slots, a rede, a descoberta, a persistência e o
//! pareamento num único [`Core`].
//!
//! É o que o app desktop (T-506) mantém no `AppState`, e o que os testes de ponta a ponta sobem
//! sem janela. Este crate existe separado do `vynpad-core` porque o `vynpad-net` depende do
//! core; a composição precisa depender dos dois.
//!
//! O que o [`Core`] resolve:
//! - **Pareamento persistente**: o [`DeviceStoreAdapter`] grava dispositivos no SQLite, e a lista
//!   de "próximos" sabe quais já são pareados.
//! - **Perfis por slot**: ao conectar, o dispositivo recebe o perfil padrão dele (ou o "Padrão");
//!   editar um perfil na UI troca o mapeamento do slot na hora, sem reconectar, e reenvia o
//!   `config` se o gyro liga ou desliga (`SlotRegistry`).
//! - **Histórico**: conexões, desconexões e recusas vão para o `connection_log`, e o "visto por
//!   último" do dispositivo é atualizado.
//! - **Confirmação de novos dispositivos** (`answer_approval`) e as ações da UI (revogar,
//!   esquecer, desconectar um slot, renomear, preferências).
//! - **Eventos**: um único canal ([`CoreEvent`]) com tudo o que a UI precisa saber.

mod config;
mod core;
mod diagnostics;
mod error;
mod event;
mod metrics;
mod registry;
mod store;

pub use config::{CoreConfig, computer_name};
pub use core::{BoundPorts, Core, PairingOffer};
pub use diagnostics::{
    DeviceSummary, Diagnostics, DiagnosticsError, LogFile, REDACTED, build_zip, redact, utc_stamp,
};
pub use error::CoreError;
pub use event::{ApprovalEvent, ApprovalPrompt, CoreEvent};
pub use metrics::{
    Limits, MetricsCollector, MetricsSnapshot, Quality, QualityThresholds, SlotMetrics,
};
pub use registry::SlotRegistry;
pub use store::{DeviceStoreAdapter, PairedSet};
