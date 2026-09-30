//! Eventos do `Core` para a UI (a T-506 os liga a eventos Tauri tipados).

use std::net::IpAddr;
use std::sync::Arc;

use padgaming_core::SlotsSnapshot;
use padgaming_net::control::ControlEvent;
use padgaming_net::discovery::NearbyDevice;
use padgaming_storage::AppSettings;

/// Pedido de confirmação de um novo dispositivo (popup de 30 s).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalPrompt {
    /// Identificador para responder com [`crate::Core::answer_approval`].
    pub request_id: u64,
    /// `device_id` do aparelho.
    pub device_id: String,
    /// Nome que ele enviou.
    pub name: String,
    /// Modelo.
    pub model: String,
    /// IP de origem.
    pub peer: IpAddr,
}

/// O ciclo de vida de um pedido de confirmação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalEvent {
    /// Há um pedido esperando resposta: a UI abre o popup.
    Requested(ApprovalPrompt),
    /// O pedido acabou (respondido ou expirou): a UI fecha o popup.
    Resolved {
        /// Qual pedido.
        request_id: u64,
        /// Foi aceito.
        accepted: bool,
    },
}

/// Tudo o que a UI precisa saber que mudou no server.
#[derive(Debug, Clone)]
pub enum CoreEvent {
    /// Os 4 slots mudaram (`slots:changed`).
    Slots(Arc<SlotsSnapshot>),
    /// As configurações mudaram (`settings:changed`).
    Settings(Arc<AppSettings>),
    /// Pareamento, sessões e estado do input (`devices:paired`, `session:state`...).
    Control(ControlEvent),
    /// A lista de dispositivos próximos mudou (`devices:discovered`).
    Nearby(Vec<NearbyDevice>),
    /// Confirmação de novo dispositivo.
    Approval(ApprovalEvent),
    /// A lista de dispositivos pareados mudou (novo, renomeado, revogado, esquecido, visto).
    DevicesChanged,
}
