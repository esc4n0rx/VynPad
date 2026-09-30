//! Escolha das interfaces de rede em que o server se anuncia.
//!
//! Por padrão só entram interfaces IPv4 privadas (RFC 1918), ativas e que não pareçam
//! virtuais (Hyper-V, WSL, `VirtualBox`, `VMware`, Docker, VPNs...). Sem isso o PC anunciaria o
//! IP de uma ponte interna que o celular não alcança. O usuário pode fixar interfaces pelo nome
//! nas Configurações ([`InterfaceChoice::Only`]).

use std::io;
use std::net::Ipv4Addr;

/// Que interfaces usar.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum InterfaceChoice {
    /// Todas as privadas, ativas e não virtuais.
    #[default]
    Auto,
    /// Só as de nome igual (sem diferenciar maiúsculas). Vale mesmo para interfaces que o modo
    /// automático consideraria virtuais: quem escolheu sabe o que quer.
    Only(Vec<String>),
}

/// Endereço IPv4 de uma interface, como o sistema a reporta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawInterface {
    /// Nome (`Wi-Fi`, `vEthernet (WSL)`, `eth0`...).
    pub name: String,
    /// Endereço IPv4.
    pub ip: Ipv4Addr,
    /// A interface está operacional.
    pub up: bool,
}

/// Interface escolhida para a descoberta.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LanInterface {
    /// Nome da interface.
    pub name: String,
    /// Endereço IPv4 (mostrado na UI para a conexão manual).
    pub ip: Ipv4Addr,
}

/// Interface para a tela de Configurações, com o motivo de ter ficado de fora do modo
/// automático.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceInfo {
    /// Nome.
    pub name: String,
    /// Endereço IPv4.
    pub ip: Ipv4Addr,
    /// O modo automático a usaria.
    pub auto: bool,
    /// Parece virtual (Hyper-V, WSL, Docker, VPN...).
    pub virtual_like: bool,
}

/// Nomes que começam assim são virtuais (comparação em minúsculas).
const VIRTUAL_PREFIXES: &[&str] = &[
    "lo",
    "veth",
    "docker",
    "br-",
    "virbr",
    "vmnet",
    "vboxnet",
    "tun",
    "tap",
    "utun",
    "wg",
    "zt",
    "tailscale",
    "ppp",
    "wsl",
];
/// Nomes que contêm isso são virtuais (comparação em minúsculas).
const VIRTUAL_PARTS: &[&str] = &[
    "vethernet",
    "hyper-v",
    "wsl",
    "virtualbox",
    "vmware",
    "vbox",
    "docker",
    "tailscale",
    "zerotier",
    "hamachi",
    "vpn",
    "loopback",
    "bluetooth",
    "npcap",
    "teredo",
    "isatap",
    "pseudo",
];

/// O nome sugere uma interface virtual, de VPN ou de loopback?
pub fn looks_virtual(name: &str) -> bool {
    let lower = name.to_lowercase();
    VIRTUAL_PREFIXES.iter().any(|p| lower.starts_with(p))
        || VIRTUAL_PARTS.iter().any(|p| lower.contains(p))
}

fn automatic(raw: &RawInterface) -> bool {
    raw.up && raw.ip.is_private() && !looks_virtual(&raw.name)
}

/// Aplica a escolha à lista de interfaces do sistema. Devolve em ordem estável (por nome e IP).
pub fn select(all: &[RawInterface], choice: &InterfaceChoice) -> Vec<LanInterface> {
    let mut chosen: Vec<LanInterface> = all
        .iter()
        .filter(|raw| match choice {
            InterfaceChoice::Auto => automatic(raw),
            InterfaceChoice::Only(names) => {
                raw.up
                    && !raw.ip.is_loopback()
                    && !raw.ip.is_link_local()
                    && names.iter().any(|n| n.eq_ignore_ascii_case(&raw.name))
            }
        })
        .map(|raw| LanInterface {
            name: raw.name.clone(),
            ip: raw.ip,
        })
        .collect();
    chosen.sort();
    chosen.dedup();
    chosen
}

/// Interfaces IPv4 do sistema.
pub fn system_interfaces() -> io::Result<Vec<RawInterface>> {
    Ok(if_addrs::get_if_addrs()?
        .into_iter()
        .filter_map(|i| {
            let up = i.is_oper_up();
            match i.addr {
                if_addrs::IfAddr::V4(v4) => Some(RawInterface {
                    up,
                    name: i.name,
                    ip: v4.ip,
                }),
                if_addrs::IfAddr::V6(_) => None,
            }
        })
        .collect())
}

/// Interfaces em uso com a escolha atual.
pub fn current(choice: &InterfaceChoice) -> io::Result<Vec<LanInterface>> {
    Ok(select(&system_interfaces()?, choice))
}

/// Todas as interfaces IPv4 ativas e não loopback, para a tela de Configurações.
pub fn available() -> io::Result<Vec<InterfaceInfo>> {
    let mut list: Vec<InterfaceInfo> = system_interfaces()?
        .iter()
        .filter(|r| r.up && !r.ip.is_loopback() && !r.ip.is_link_local())
        .map(|r| InterfaceInfo {
            name: r.name.clone(),
            ip: r.ip,
            auto: automatic(r),
            virtual_like: looks_virtual(&r.name),
        })
        .collect();
    list.sort_by(|a, b| (&a.name, a.ip).cmp(&(&b.name, b.ip)));
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(name: &str, ip: [u8; 4], up: bool) -> RawInterface {
        RawInterface {
            name: name.into(),
            ip: Ipv4Addr::from(ip),
            up,
        }
    }

    /// Um PC com Wi-Fi, WSL, Hyper-V, Docker e VPN.
    fn busy_pc() -> Vec<RawInterface> {
        vec![
            raw("Wi-Fi", [192, 168, 0, 10], true),
            raw("vEthernet (WSL)", [172, 24, 0, 1], true),
            raw("vEthernet (Default Switch)", [172, 30, 16, 1], true),
            raw("VirtualBox Host-Only Network", [192, 168, 56, 1], true),
            raw("Tailscale", [100, 64, 1, 2], true),
            raw("Ethernet 2", [10, 8, 0, 5], false),
            raw("Loopback Pseudo-Interface 1", [127, 0, 0, 1], true),
            raw("Ethernet", [169, 254, 3, 4], true),
            raw("Ethernet 3", [8, 8, 4, 4], true),
        ]
    }

    #[test]
    fn auto_picks_only_the_real_private_active_interface() {
        let chosen = select(&busy_pc(), &InterfaceChoice::Auto);
        assert_eq!(
            chosen,
            [LanInterface {
                name: "Wi-Fi".into(),
                ip: Ipv4Addr::new(192, 168, 0, 10)
            }]
        );
    }

    #[test]
    fn linux_virtual_names_are_recognised() {
        for name in [
            "lo",
            "docker0",
            "br-3f2a",
            "veth1a2b",
            "virbr0",
            "tun0",
            "tap1",
            "wg0",
            "tailscale0",
            "zt5u4xyz",
            "vmnet8",
        ] {
            assert!(looks_virtual(name), "{name}");
        }
        for name in [
            "eth0",
            "enp3s0",
            "wlan0",
            "wlp2s0",
            "Wi-Fi",
            "Ethernet",
            "Ethernet 2",
        ] {
            assert!(!looks_virtual(name), "{name}");
        }
    }

    #[test]
    fn several_real_interfaces_are_all_used_in_a_stable_order() {
        let all = [
            raw("wlan0", [192, 168, 1, 20], true),
            raw("eth0", [10, 0, 0, 7], true),
        ];
        let names: Vec<_> = select(&all, &InterfaceChoice::Auto)
            .into_iter()
            .map(|i| i.name)
            .collect();
        assert_eq!(names, ["eth0", "wlan0"]);
    }

    #[test]
    fn an_explicit_choice_wins_over_the_virtual_filter_but_not_over_down_or_loopback() {
        let only = InterfaceChoice::Only(vec![
            "vethernet (wsl)".into(), // sem diferenciar maiúsculas
            "Ethernet 2".into(),      // desligada
            "Loopback Pseudo-Interface 1".into(),
        ]);
        let chosen = select(&busy_pc(), &only);
        assert_eq!(
            chosen.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
            ["vEthernet (WSL)"]
        );
        assert!(select(&busy_pc(), &InterfaceChoice::Only(vec![])).is_empty());
    }

    #[test]
    fn available_never_reports_loopback_or_a_virtual_interface_as_automatic() {
        let list = available().unwrap();
        assert!(
            list.iter()
                .all(|i| !i.ip.is_loopback() && !i.ip.is_link_local())
        );
        assert!(list.iter().all(|i| !(i.auto && i.virtual_like)));
    }
}
