//! Parâmetros para montar o `Core`.

use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::sync::Arc;

use padgaming_core::SlotConfig;
use padgaming_driver::VirtualPadDriver;
use padgaming_net::control::ControlConfig;
use padgaming_net::discovery::InterfaceChoice;
use padgaming_net::pairing::PairingConfig;
use padgaming_storage::Ports;

/// Como o `Core` é montado. [`CoreConfig::new`] dá os padrões de produção; os campos públicos
/// existem para os testes (portas 0, sem mDNS, relógio dos timeouts encurtado).
pub struct CoreConfig {
    /// Pasta de dados: `padgaming.db`, certificado e chave do server.
    pub data_dir: PathBuf,
    /// O driver de controles virtuais. `None` usa o do sistema (`ViGEmBus` no Windows).
    pub driver: Option<Arc<dyn VirtualPadDriver>>,
    /// Nome do server na rede. `None` usa o nome do computador.
    pub server_name: Option<String>,
    /// Endereço em que as portas escutam. O padrão é qualquer interface.
    pub bind_ip: IpAddr,
    /// Substitui as portas das configurações (testes: porta 0 = uma livre).
    pub ports_override: Option<Ports>,
    /// Liga o mDNS (anúncio e lista de próximos). O broadcast fica sempre ligado.
    pub mdns: bool,
    /// Interfaces em que anunciar.
    pub interfaces: InterfaceChoice,
    /// Timeouts do canal de controle.
    pub control: ControlConfig,
    /// Reserva de slot após a queda.
    pub slots: SlotConfig,
    /// Validades de token e PIN, limites de tentativa.
    pub pairing: PairingConfig,
}

impl CoreConfig {
    /// Padrões de produção para a pasta de dados `data_dir`.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            driver: None,
            server_name: None,
            bind_ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            ports_override: None,
            mdns: true,
            interfaces: InterfaceChoice::Auto,
            control: ControlConfig::default(),
            slots: SlotConfig::default(),
            pairing: PairingConfig::default(),
        }
    }
}

/// O nome do computador, para o nome padrão do server.
pub fn computer_name() -> String {
    ["COMPUTERNAME", "HOSTNAME"]
        .into_iter()
        .find_map(|key| std::env::var(key).ok())
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "PadGaming".to_owned())
}
