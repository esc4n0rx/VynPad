//! Descoberta ponta a ponta: o `padgaming-sim` procura o server por broadcast e por mDNS, e o
//! server lista o client anunciado em "dispositivos próximos".
//!
//! Os testes de mDNS dependem de multicast na rede local e ficam `#[ignore]` (como os de
//! hardware): `cargo test -p padgaming-sim --test discovery -- --ignored`.

use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use padgaming_core::SlotManager;
use padgaming_driver::MockDriver;
use padgaming_net::discovery::{
    Discovery, DiscoveryConfig, InterfaceChoice, NearbyDevice, ServerInfo,
};
use padgaming_protocol::control::Uuid;
use padgaming_sim::discover::{self, ClientAnnouncement, Via};

const SERVER_ID: &str = "5b0c8a1e-2f4d-4c7a-9e1b-0d3f6a7b8c9d";
const CLIENT_ID: &str = "00000000-0000-4000-8000-0000000000aa";

fn info(name: &str) -> ServerInfo {
    ServerInfo {
        id: Uuid::parse(SERVER_ID).unwrap(),
        name: name.into(),
        tcp_port: 47800,
        udp_port: 47801,
        os: ServerInfo::current_os().into(),
        free: 4,
        fingerprint: [0x3f; 32],
    }
}

type Seen = Arc<Mutex<Vec<Vec<NearbyDevice>>>>;

fn start(name: &str, mdns: bool) -> (Discovery, Seen) {
    let seen: Seen = Arc::default();
    let sink = Arc::clone(&seen);
    let discovery = Discovery::start(
        &DiscoveryConfig {
            interfaces: InterfaceChoice::Auto,
            broadcast_bind: "127.0.0.1:0".parse().unwrap(),
            mdns,
        },
        &info(name),
        Arc::new(|_| false),
        Arc::new(move |list| sink.lock().unwrap().push(list)),
    )
    .unwrap();
    (discovery, seen)
}

fn wait_until(what: &str, limit: Duration, mut cond: impl FnMut() -> bool) -> Duration {
    let start = Instant::now();
    while start.elapsed() < limit {
        if cond() {
            return start.elapsed();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("não aconteceu em {limit:?}: {what}");
}

#[test]
fn the_sim_finds_the_server_by_broadcast_and_sees_slot_changes() {
    let (discovery, _) = start("PC-Sala", false);
    let target = discovery.broadcast_addr().unwrap();

    let found = discover::by_broadcast(&[target], Duration::from_millis(400)).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    let server = &found[0];
    assert_eq!(server.id, SERVER_ID);
    assert_eq!(server.name, "PC-Sala");
    assert_eq!((server.tcp, server.udp, server.free), (47800, 47801, 4));
    assert_eq!(server.fp, "3f3f3f3f3f3f3f3f");
    assert_eq!(server.via, [Via::Broadcast]);
    assert_eq!(server.addrs, [Ipv4Addr::LOCALHOST]);

    // O `free` acompanha os slots.
    let slots = SlotManager::new(Arc::new(MockDriver::default()));
    let follower = discovery.follow_slots(&slots).unwrap();
    slots
        .assign(&Uuid::parse(CLIENT_ID).unwrap(), 7, None)
        .unwrap();
    wait_until("free = 3", Duration::from_secs(2), || {
        discover::by_broadcast(&[target], Duration::from_millis(100))
            .unwrap()
            .first()
            .is_some_and(|f| f.free == 3)
    });
    slots.release(7);
    wait_until("free = 4", Duration::from_secs(2), || {
        discover::by_broadcast(&[target], Duration::from_millis(100))
            .unwrap()
            .first()
            .is_some_and(|f| f.free == 4)
    });
    discovery.shutdown();
    drop(slots);
    follower.join().unwrap();
}

#[test]
fn nothing_answers_when_nothing_listens() {
    let (discovery, _) = start("PC", false);
    let target = discovery.broadcast_addr().unwrap();
    discovery.shutdown();
    let found = discover::by_broadcast(&[target], Duration::from_millis(300)).unwrap();
    assert!(found.is_empty());
}

#[test]
fn a_renamed_server_answers_with_the_new_name() {
    let (discovery, _) = start("Antigo", false);
    let target = discovery.broadcast_addr().unwrap();
    discovery.set_name("Novo");
    let found = discover::by_broadcast(&[target], Duration::from_millis(300)).unwrap();
    assert_eq!(found[0].name, "Novo");
}

#[test]
#[ignore = "precisa de multicast na rede local"]
fn the_sim_finds_the_server_by_mdns_and_by_broadcast_as_one_row() {
    let (discovery, _) = start("PC-Teste-mDNS", true);
    if discovery.local_addresses().is_empty() {
        eprintln!("sem interface de LAN utilizável: teste pulado");
        return;
    }
    let target = discovery.broadcast_addr().unwrap();

    let mut results = discover::Results::default();
    for found in discover::by_mdns(Duration::from_secs(3)).unwrap() {
        results.add(found);
    }
    for found in discover::by_broadcast(&[target], Duration::from_millis(300)).unwrap() {
        results.add(found);
    }
    let list = results.into_list();
    let server = list
        .iter()
        .find(|f| f.id == SERVER_ID)
        .unwrap_or_else(|| panic!("server não encontrado: {list:?}"));
    assert_eq!(server.name, "PC-Teste-mDNS");
    assert_eq!((server.tcp, server.udp), (47800, 47801));
    assert_eq!(server.fp, "3f3f3f3f3f3f3f3f");
    assert!(server.via.contains(&Via::Mdns), "{server:?}");
    assert!(server.via.contains(&Via::Broadcast));
    // Só o IP da LAN foi anunciado, não os de WSL/Hyper-V.
    for addr in &server.addrs {
        assert!(
            discovery
                .local_addresses()
                .iter()
                .any(|i| i.ip == *addr || *addr == Ipv4Addr::LOCALHOST),
            "{addr} não é de uma interface escolhida"
        );
    }
    discovery.shutdown();
}

#[test]
#[ignore = "precisa de multicast na rede local"]
fn an_announced_client_shows_up_within_3s_and_leaves_within_15s() {
    let (discovery, seen) = start("PC-Teste-mDNS", true);
    let Some(ip) = discover::guess_lan_ip() else {
        eprintln!("sem IPv4 de LAN: teste pulado");
        return;
    };
    let announcement =
        ClientAnnouncement::start(CLIENT_ID, "Pixel do Teste", "Pixel 8", ip, 50000).unwrap();

    let appeared = wait_until("client na lista", Duration::from_secs(3), || {
        discovery
            .nearby()
            .iter()
            .any(|d| d.id.as_str() == CLIENT_ID)
    });
    eprintln!("apareceu em {appeared:?}");
    let device = discovery
        .nearby()
        .into_iter()
        .find(|d| d.id.as_str() == CLIENT_ID)
        .unwrap();
    assert_eq!(device.name, "Pixel do Teste");
    assert_eq!(device.model, "Pixel 8");
    assert_eq!(device.ip, ip);
    assert!(!device.paired);
    // A UI foi avisada, com o debounce.
    wait_until("evento devices:discovered", Duration::from_secs(2), || {
        seen.lock()
            .unwrap()
            .iter()
            .any(|l| l.iter().any(|d| d.id.as_str() == CLIENT_ID))
    });

    // Sai da rede com o "goodbye".
    announcement.stop();
    let left = wait_until("client some da lista", Duration::from_secs(15), || {
        discovery
            .nearby()
            .iter()
            .all(|d| d.id.as_str() != CLIENT_ID)
    });
    eprintln!("sumiu em {left:?}");
    discovery.shutdown();
}

#[test]
#[ignore = "precisa de multicast na rede local"]
fn a_client_that_vanishes_without_goodbye_leaves_within_15s() {
    let (discovery, _) = start("PC-Teste-mDNS", true);
    let Some(ip) = discover::guess_lan_ip() else {
        eprintln!("sem IPv4 de LAN: teste pulado");
        return;
    };
    let announcement =
        ClientAnnouncement::start(CLIENT_ID, "Pixel Morto", "Pixel 8", ip, 50000).unwrap();
    wait_until("client na lista", Duration::from_secs(3), || {
        discovery
            .nearby()
            .iter()
            .any(|d| d.id.as_str() == CLIENT_ID)
    });

    announcement.vanish();
    let left = wait_until("client some da lista", Duration::from_secs(15), || {
        discovery
            .nearby()
            .iter()
            .all(|d| d.id.as_str() != CLIENT_ID)
    });
    eprintln!("sumiu sem goodbye em {left:?}");
    discovery.shutdown();
}
