//! Server de descoberta de desenvolvimento: anuncia por mDNS e broadcast e imprime os
//! dispositivos próximos. Use com `padgaming-sim discover` / `padgaming-sim announce`.
//!
//! ```text
//! cargo run -p padgaming-net --example discovery_server
//! ```

use std::sync::Arc;
use std::time::Duration;

use padgaming_net::discovery::{Discovery, DiscoveryConfig, ServerInfo, interfaces};
use padgaming_protocol::control::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("interfaces IPv4 ativas (auto = usada pelo anúncio):");
    for i in interfaces::available()? {
        println!(
            "  {:<40} {:<16} auto={} virtual={}",
            i.name, i.ip, i.auto, i.virtual_like
        );
    }
    let info = ServerInfo {
        id: Uuid::parse("5b0c8a1e-2f4d-4c7a-9e1b-0d3f6a7b8c9d").map_err(|e| e.to_string())?,
        name: "PadGaming Dev".into(),
        tcp_port: 47800,
        udp_port: 47801,
        os: ServerInfo::current_os().into(),
        free: 4,
        fingerprint: [0x3f; 32],
    };
    let discovery = Discovery::start(
        &DiscoveryConfig::default(),
        &info,
        Arc::new(|_| false),
        Arc::new(|list| {
            println!("próximos: {}", list.len());
            for d in list {
                println!(
                    "  {} ({}) {} visto há {:?}",
                    d.name, d.model, d.ip, d.seen_ago
                );
            }
        }),
    )
    .map_err(|e| e.to_string())?;
    println!(
        "anunciando em {:?}; Ctrl+C para sair",
        discovery.local_addresses()
    );
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}
