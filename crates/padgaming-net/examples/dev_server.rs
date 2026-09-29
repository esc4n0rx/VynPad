//! Server de desenvolvimento (M1): escuta o input UDP com as sessões `1..=4` abertas e a chave
//! pública de desenvolvimento, para testar com `padgaming-sim --insecure-dev`.
//!
//! ```text
//! cargo run -p padgaming-net --example dev_server              # controles virtuais reais (ViGEmBus)
//! cargo run -p padgaming-net --example dev_server -- --mock    # só imprime, sem tocar no sistema
//! cargo run -p padgaming-net --example dev_server -- --port 50000 --profile meu-perfil.json
//! ```
//!
//! **Sem autenticação nenhuma**: qualquer um na rede que conheça a chave (pública) controla o
//! PC. Só existe em builds de debug e é só para desenvolvimento.

/// Tempo sem pacotes depois do qual uma sessão de desenvolvimento é reaberta.
#[cfg(debug_assertions)]
const IDLE_RESET: std::time::Duration = std::time::Duration::from_millis(1500);

#[cfg(debug_assertions)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;
    use std::time::Duration;

    use padgaming_core::SlotManager;
    use padgaming_driver::{MockDriver, VirtualPadDriver};
    use padgaming_mapping::{CompiledProfile, Profile, ProfileCell};
    use padgaming_net::input::{
        DEFAULT_PORT, DEV_KEY, DEV_SESSION_ID, InputConfig, InputServer, SessionSpec,
    };
    use padgaming_protocol::Secret;
    use padgaming_protocol::control::Uuid;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().any(|a| a == name);
    let value = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
    };
    let port: u16 = value("--port").map_or(Ok(DEFAULT_PORT), |p| p.parse())?;
    let profile = match value("--profile") {
        Some(path) => Profile::from_json(&std::fs::read_to_string(path)?)?,
        None => Profile::standard(),
    };

    let driver: Arc<dyn VirtualPadDriver> = if flag("--mock") {
        Arc::new(MockDriver::default())
    } else {
        Arc::from(padgaming_driver::default_driver())
    };
    println!("driver: {:?}", driver.status());
    let slots = Arc::new(SlotManager::new(Arc::clone(&driver)));
    let cell = Arc::new(ProfileCell::new(CompiledProfile::compile(&profile)?));

    let config = InputConfig {
        bind: std::net::SocketAddr::from(([0, 0, 0, 0], port)),
        ..InputConfig::default()
    };
    let server = InputServer::start(config, Arc::clone(&slots))?;
    let spec = |session_id: u32| SessionSpec {
        session_id,
        key: Secret::new(DEV_KEY),
        profile: Arc::clone(&cell),
    };
    let mut activities = Vec::new();
    for i in 0..4u32 {
        let session_id = DEV_SESSION_ID + i;
        let device = Uuid::parse(&format!("00000000-0000-4000-8000-00000000000{i}"))?;
        let slot = slots.assign(&device, session_id, None)?;
        activities.push(server.add_session(spec(session_id)));
        println!("sessão {session_id} -> slot {}", slot.index());
    }
    println!(
        "escutando em {} (perfil \"{}\"); Ctrl+C para sair\n",
        server.local_addr(),
        profile.name
    );

    let stats = Arc::clone(server.stats());
    let mut last = stats.snapshot();
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let now = stats.snapshot();
        println!(
            "inputs/s {:>5}  pings/s {:>3}  descartes {:>4}  erros do driver {}  p99 {:?}  max {:?}",
            now.inputs - last.inputs,
            now.pings - last.pings,
            now.dropped(),
            now.submit_errors,
            stats.processing.percentile(99.0),
            stats.processing.max(),
        );
        last = now;

        // Cada execução do simulador recomeça o `seq` em 0, e o server recusa `seq` antigo
        // (anti-replay). Quando uma sessão fica ociosa, ela é reaberta com a janela zerada, para
        // rodar o simulador de novo sem reiniciar o server. (No app de verdade, cada conexão
        // ganha uma sessão nova no handshake.)
        for (i, activity) in activities.iter_mut().enumerate() {
            if activity.peer().is_some()
                && activity.idle_for(std::time::Instant::now()) > IDLE_RESET
            {
                let session_id = DEV_SESSION_ID + u32::try_from(i)?;
                *activity = server.add_session(spec(session_id));
                println!("sessão {session_id} ociosa: reaberta");
            }
        }
    }
}

#[cfg(not(debug_assertions))]
fn main() {
    eprintln!("o dev_server usa a chave pública de desenvolvimento e só existe em builds de debug");
}
