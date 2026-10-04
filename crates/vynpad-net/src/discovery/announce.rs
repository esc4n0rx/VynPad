//! Anúncio do server por mDNS/DNS-SD: `_vynpad._tcp.local.` (PROTOCOLO §3.1).

use std::collections::BTreeSet;

use mdns_sd::{IfKind, ServiceDaemon, ServiceInfo};
use vynpad_protocol::discovery::SERVER_SERVICE;

use super::DiscoveryError;
use super::info::ServerInfo;
use super::interfaces::LanInterface;

/// Comprimento máximo do tipo de serviço aceito pelo daemon. O padrão do `mdns-sd` (15, da
/// RFC 6763) recusaria `_vynpad-client` (17); os nomes vêm do PROTOCOLO §3.1.
const SERVICE_NAME_LEN_MAX: u8 = 30;

/// Cria o daemon mDNS restrito às `interfaces` (as virtuais ficam de fora).
pub fn new_daemon(interfaces: &[LanInterface]) -> Result<ServiceDaemon, DiscoveryError> {
    let daemon = ServiceDaemon::new()?;
    daemon.set_service_name_len_max(SERVICE_NAME_LEN_MAX)?;
    daemon.disable_interface(IfKind::All)?;
    for interface in interfaces {
        daemon.enable_interface(IfKind::Name(interface.name.clone()))?;
    }
    Ok(daemon)
}

/// Ajusta as interfaces do `daemon` de `before` para `after` (Wi-Fi trocado, cabo plugado...).
pub fn switch_interfaces(
    daemon: &ServiceDaemon,
    before: &[LanInterface],
    after: &[LanInterface],
) -> Result<(), DiscoveryError> {
    let names = |list: &[LanInterface]| -> BTreeSet<String> {
        list.iter().map(|i| i.name.clone()).collect()
    };
    let (old, new) = (names(before), names(after));
    for gone in old.difference(&new) {
        daemon.disable_interface(IfKind::Name(gone.clone()))?;
    }
    for added in new.difference(&old) {
        daemon.enable_interface(IfKind::Name(added.clone()))?;
    }
    Ok(())
}

/// Monta o `ServiceInfo` do server para as `interfaces`.
pub fn service_info(
    info: &ServerInfo,
    interfaces: &[LanInterface],
) -> Result<ServiceInfo, DiscoveryError> {
    if interfaces.is_empty() {
        return Err(DiscoveryError::NoInterface);
    }
    // `.local.` exige um nome de host; um sufixo do id evita colidir com outros VynPad.
    let host = format!("vynpad-{}.local.", &info.id.as_str()[..8]);
    let addresses: Vec<String> = interfaces.iter().map(|i| i.ip.to_string()).collect();
    let mut service = ServiceInfo::new(
        SERVER_SERVICE,
        &info.wire_name(),
        &host,
        addresses.join(",").as_str(),
        info.tcp_port,
        &info.txt()[..],
    )?
    .enable_addr_auto();
    service.set_interfaces(
        interfaces
            .iter()
            .map(|i| IfKind::Name(i.name.clone()))
            .collect(),
    );
    Ok(service)
}

/// O anúncio do server. Chame [`Announcer::update`] quando `free` (ou o nome) mudar.
pub struct Announcer {
    daemon: ServiceDaemon,
    fullname: String,
}

impl Announcer {
    /// Registra o serviço no `daemon`.
    pub fn start(
        daemon: &ServiceDaemon,
        info: &ServerInfo,
        interfaces: &[LanInterface],
    ) -> Result<Self, DiscoveryError> {
        let service = service_info(info, interfaces)?;
        let fullname = service.get_fullname().to_owned();
        daemon.register(service)?;
        tracing::info!(%fullname, port = info.tcp_port, "server anunciado por mDNS");
        Ok(Self {
            daemon: daemon.clone(),
            fullname,
        })
    }

    /// Reanuncia com os dados novos (o TXT muda, o nome do serviço fica).
    pub fn update(
        &self,
        info: &ServerInfo,
        interfaces: &[LanInterface],
    ) -> Result<(), DiscoveryError> {
        self.daemon.register(service_info(info, interfaces)?)?;
        Ok(())
    }

    /// Retira o anúncio (com o "goodbye" do mDNS, para os clients atualizarem na hora).
    pub fn shutdown(self) {
        if let Ok(done) = self.daemon.unregister(&self.fullname) {
            // Espera curta: o goodbye sai pela rede antes de o daemon parar.
            let _ = done.recv_timeout(std::time::Duration::from_millis(500));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::super::info::tests::sample;
    use super::*;

    fn lan(name: &str, last: u8) -> LanInterface {
        LanInterface {
            name: name.into(),
            ip: Ipv4Addr::new(192, 168, 0, last),
        }
    }

    #[test]
    fn the_service_carries_the_txt_port_and_the_chosen_addresses() {
        let service = service_info(&sample(), &[lan("Wi-Fi", 10), lan("Ethernet", 11)]).unwrap();
        assert_eq!(service.get_type(), "_vynpad._tcp.local.");
        assert_eq!(service.get_fullname(), "PC-Sala._vynpad._tcp.local.");
        assert_eq!(service.get_hostname(), "vynpad-5b0c8a1e.local.");
        assert_eq!(service.get_port(), 47800);
        assert_eq!(service.get_addresses_v4().len(), 2);
        assert_eq!(service.get_property_val_str("free"), Some("3"));
        assert_eq!(service.get_property_val_str("udp"), Some("47801"));
        assert_eq!(service.get_property_val_str("fp"), Some("abababababababab"));
        assert!(service.is_addr_auto());
    }

    #[test]
    fn without_a_usable_interface_there_is_nothing_to_announce() {
        assert!(matches!(
            service_info(&sample(), &[]),
            Err(DiscoveryError::NoInterface)
        ));
    }
}
