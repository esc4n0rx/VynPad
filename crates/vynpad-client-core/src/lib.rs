//! Núcleo do client `VynPad` (D-26): o app Compose chama este crate por `UniFFI` e, assim, usa o
//! mesmo `vynpad-protocol` do server, sem reimplementar o protocolo em Kotlin.
//!
//! Esta primeira camada cobre o que a UI precisa para as próximas etapas: a sessão UDP
//! ([`ClientSession`]), a leitura da URI de pareamento e a leitura da resposta de descoberta.
//! Sockets, TLS e o laço de envio entram nas etapas seguintes (T-903 a T-906).

// O scaffolding gerado pelo UniFFI usa `unsafe`; o código escrito à mão deste crate não.
#![allow(unsafe_code)]

mod control;
mod error;
mod pairing;
mod pump;
mod scan;
mod session;
mod types;

pub use control::{
    ConfigInfo, ControlConnection, Credential, DeviceInfo, SessionListener, TrustPolicy,
    WelcomeInfo,
};
pub use error::CoreError;
pub use pairing::{
    DiscoveredServer, PairingInfo, discovery_port, discovery_request, parse_discovery_response,
    parse_pairing_uri,
};
pub use pump::{InputPump, PumpListener};
pub use scan::{BroadcastScanner, ScanListener};
pub use session::ClientSession;
pub use types::{GamepadInput, MotionSample, ReceivedPacket};

uniffi::setup_scaffolding!();

/// Versão deste núcleo (a do workspace).
#[uniffi::export]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Versão do protocolo UDP implementada (`ver` do cabeçalho).
#[uniffi::export]
pub fn protocol_version() -> u8 {
    vynpad_protocol::VERSION
}
