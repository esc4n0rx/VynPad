//! Estado compartilhado do backend, injetado nos commands do Tauri.

use std::sync::Arc;

use padgaming_server::{Core, CoreConfig, CoreError};

use crate::live::LiveInput;

/// O que os commands enxergam.
pub struct AppState {
    /// O server inteiro (banco, slots, rede, descoberta, pareamento).
    pub core: Arc<Core>,
    /// Assinaturas do `input:live`.
    pub live: Arc<LiveInput>,
    #[cfg(windows)]
    pub xinput: Option<padgaming_driver::xinput::XInputProbe>,
}

impl AppState {
    /// Sobe o `Core` com `config`.
    pub async fn start(config: CoreConfig) -> Result<Self, CoreError> {
        Ok(Self::with_core(Core::start(config).await?))
    }

    /// Estado em cima de um `Core` já iniciado.
    pub fn with_core(core: Core) -> Self {
        Self {
            core: Arc::new(core),
            live: Arc::new(LiveInput::default()),
            #[cfg(windows)]
            xinput: padgaming_driver::xinput::XInputProbe::load(),
        }
    }
}
