//! Descoberta do lado do client (PROTOCOLO §3): procura servers por mDNS e por broadcast, e
//! anuncia um client simulado para o server listar em "dispositivos próximos".

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use anyhow::Context;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use vynpad_protocol::discovery::{
    BROADCAST_REQUEST, CLIENT_SERVICE, DiscoveryResponse, SERVER_SERVICE, txt,
};

/// Comprimento máximo do tipo de serviço no daemon (o padrão de 15 recusa `_vynpad-client`).
const SERVICE_NAME_LEN_MAX: u8 = 30;

/// Por onde o server foi encontrado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Via {
    /// mDNS / DNS-SD.
    Mdns,
    /// Broadcast `VPDISC1`.
    Broadcast,
}

/// Um server encontrado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// `id` do server (UUID).
    pub id: String,
    /// Nome exibido.
    pub name: String,
    /// Endereços IPv4 pelos quais respondeu.
    pub addrs: Vec<Ipv4Addr>,
    /// Porta TCP de controle.
    pub tcp: u16,
    /// Porta UDP de input.
    pub udp: u16,
    /// `windows` ou `linux`.
    pub os: String,
    /// Slots livres.
    pub free: u8,
    /// 8 primeiros bytes do fingerprint em hex (o usuário confere com a tela do server).
    pub fp: String,
    /// Como foi encontrado (pode ser os dois).
    pub via: Vec<Via>,
}

/// Junta os achados por `id` (o mesmo server por mDNS e por broadcast vira uma linha só).
#[derive(Debug, Default)]
pub struct Results {
    by_id: BTreeMap<String, Found>,
}

impl Results {
    /// Registra um achado.
    pub fn add(&mut self, found: Found) {
        match self.by_id.get_mut(&found.id) {
            None => {
                self.by_id.insert(found.id.clone(), found);
            }
            Some(known) => {
                for addr in found.addrs {
                    if !known.addrs.contains(&addr) {
                        known.addrs.push(addr);
                    }
                }
                for via in found.via {
                    if !known.via.contains(&via) {
                        known.via.push(via);
                    }
                }
                known.addrs.sort();
                known.via.sort();
            }
        }
    }

    /// Os servers, ordenados por nome.
    pub fn into_list(self) -> Vec<Found> {
        let mut list: Vec<Found> = self.by_id.into_values().collect();
        list.sort_by(|a, b| (&a.name, &a.id).cmp(&(&b.name, &b.id)));
        list
    }
}

/// Endereços de destino do broadcast: o limitado e o dirigido de cada interface IPv4 (o Windows
/// só manda o limitado por uma das interfaces).
pub fn broadcast_targets(port: u16) -> Vec<SocketAddr> {
    let mut targets = vec![SocketAddr::from((Ipv4Addr::BROADCAST, port))];
    if let Ok(interfaces) = if_addrs::get_if_addrs() {
        for interface in interfaces {
            if let if_addrs::IfAddr::V4(v4) = interface.addr
                && let Some(broadcast) = v4.broadcast
                && !v4.ip.is_loopback()
            {
                targets.push(SocketAddr::from((broadcast, port)));
            }
        }
    }
    targets.sort();
    targets.dedup();
    targets
}

/// Envia `VPDISC1` para `targets` e coleta as respostas por `wait`.
pub fn by_broadcast(targets: &[SocketAddr], wait: Duration) -> anyhow::Result<Vec<Found>> {
    let socket = UdpSocket::bind("0.0.0.0:0").context("não consegui abrir o socket UDP")?;
    socket.set_broadcast(true)?;
    for target in targets {
        // Uma interface sem rota falha; as outras seguem.
        let _ = socket.send_to(BROADCAST_REQUEST, target);
    }
    let deadline = Instant::now() + wait;
    let mut found = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        socket.set_read_timeout(Some(left))?;
        let Ok((n, from)) = socket.recv_from(&mut buf) else {
            break;
        };
        let (Ok(response), IpAddr::V4(ip)) = (DiscoveryResponse::from_json(&buf[..n]), from.ip())
        else {
            continue; // Resposta inválida: a descoberta é só uma dica, ignora.
        };
        found.push(Found {
            id: response.id.as_str().to_owned(),
            name: response.name,
            addrs: vec![ip],
            tcp: response.tcp,
            udp: response.udp,
            os: response.os,
            free: response.free,
            fp: response.fp,
            via: vec![Via::Broadcast],
        });
    }
    Ok(found)
}

/// Procura `_vynpad._tcp` por `wait`.
pub fn by_mdns(wait: Duration) -> anyhow::Result<Vec<Found>> {
    let daemon = ServiceDaemon::new().context("mDNS indisponível")?;
    daemon.set_service_name_len_max(SERVICE_NAME_LEN_MAX)?;
    let events = daemon.browse(SERVER_SERVICE)?;
    let deadline = Instant::now() + wait;
    let mut found = Vec::new();
    while let Some(left) =
        Some(deadline.saturating_duration_since(Instant::now())).filter(|d| !d.is_zero())
    {
        match events.recv_timeout(left) {
            Ok(ServiceEvent::ServiceResolved(resolved)) => {
                let prop = |k: &str| resolved.get_property_val_str(k).map(str::to_owned);
                let (Some(id), Some(fp)) = (prop(txt::ID), prop(txt::FP)) else {
                    continue;
                };
                let mut addrs: Vec<Ipv4Addr> = resolved.get_addresses_v4().into_iter().collect();
                addrs.sort();
                found.push(Found {
                    name: prop(txt::NAME).unwrap_or_else(|| resolved.get_fullname().to_owned()),
                    id,
                    addrs,
                    tcp: resolved.get_port(),
                    udp: prop(txt::UDP).and_then(|v| v.parse().ok()).unwrap_or(0),
                    os: prop(txt::OS).unwrap_or_default(),
                    free: prop(txt::FREE).and_then(|v| v.parse().ok()).unwrap_or(0),
                    fp,
                    via: vec![Via::Mdns],
                });
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    let _ = daemon.shutdown();
    Ok(found)
}

/// Um client simulado anunciado por mDNS até ser derrubado.
pub struct ClientAnnouncement {
    daemon: ServiceDaemon,
    fullname: String,
}

impl ClientAnnouncement {
    /// Anuncia `_vynpad-client._tcp` com o TXT `id`, `name` e `model` (PROTOCOLO §3.1).
    /// `port` é só informativa (o server nunca conecta nela).
    pub fn start(
        id: &str,
        name: &str,
        model: &str,
        ip: Ipv4Addr,
        port: u16,
    ) -> anyhow::Result<Self> {
        let daemon = ServiceDaemon::new().context("mDNS indisponível")?;
        daemon.set_service_name_len_max(SERVICE_NAME_LEN_MAX)?;
        let host = format!("sim-{}.local.", &id[..id.len().min(8)]);
        let properties = [(txt::ID, id), (txt::NAME, name), (txt::MODEL, model)];
        let service = ServiceInfo::new(
            CLIENT_SERVICE,
            name,
            &host,
            ip.to_string().as_str(),
            port,
            &properties[..],
        )?;
        let fullname = service.get_fullname().to_owned();
        daemon.register(service)?;
        Ok(Self { daemon, fullname })
    }

    /// Some sem se despedir (simula o app morto ou o Wi-Fi caindo): para o daemon sem o
    /// "goodbye". O server só percebe pela expiração.
    pub fn vanish(self) {
        let _ = self.daemon.shutdown();
    }

    /// Retira o anúncio (com o "goodbye") e para o daemon.
    pub fn stop(self) {
        if let Ok(done) = self.daemon.unregister(&self.fullname) {
            let _ = done.recv_timeout(Duration::from_millis(500));
        }
        let _ = self.daemon.shutdown();
    }
}

/// O IPv4 privado mais provável de ser o da LAN (o primeiro que não parece virtual).
pub fn guess_lan_ip() -> Option<Ipv4Addr> {
    if_addrs::get_if_addrs()
        .ok()?
        .into_iter()
        .filter(if_addrs::Interface::is_oper_up)
        .find_map(|i| match i.addr {
            if_addrs::IfAddr::V4(v4)
                if v4.ip.is_private() && !i.name.to_lowercase().contains("vethernet") =>
            {
                Some(v4.ip)
            }
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(id: &str, via: Via, ip: [u8; 4]) -> Found {
        Found {
            id: id.into(),
            name: "PC-Sala".into(),
            addrs: vec![Ipv4Addr::from(ip)],
            tcp: 47800,
            udp: 47801,
            os: "windows".into(),
            free: 3,
            fp: "3fa1c2d4e5f60718".into(),
            via: vec![via],
        }
    }

    #[test]
    fn the_same_server_by_mdns_and_broadcast_is_one_row_with_both_sources() {
        let mut results = Results::default();
        results.add(found("a", Via::Broadcast, [192, 168, 0, 10]));
        results.add(found("a", Via::Mdns, [192, 168, 0, 10]));
        results.add(found("a", Via::Mdns, [10, 0, 0, 5]));
        results.add(found("b", Via::Mdns, [192, 168, 0, 11]));
        let list = results.into_list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].via, [Via::Mdns, Via::Broadcast]);
        assert_eq!(
            list[0].addrs,
            [Ipv4Addr::new(10, 0, 0, 5), Ipv4Addr::new(192, 168, 0, 10)]
        );
    }

    #[test]
    fn broadcast_targets_always_include_the_limited_broadcast() {
        let targets = broadcast_targets(47802);
        assert!(targets.contains(&SocketAddr::from(([255, 255, 255, 255], 47802))));
        assert!(targets.iter().all(|t| t.port() == 47802));
    }
}
