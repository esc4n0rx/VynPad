//! Checa o driver de emulação e mostra o que a UI mostraria.
//!
//! ```text
//! cargo run -p padgaming-driver --example check_driver                 # estado real
//! cargo run -p padgaming-driver --example check_driver -- --open       # abre a página se faltar
//! cargo run -p padgaming-driver --example check_driver -- --simulate missing|stopped|outdated
//! ```

#[cfg(windows)]
fn main() {
    use padgaming_driver::setup::{self, ServiceState};
    use padgaming_driver::{DriverStatus, VIGEM_DOWNLOAD_URL, VigemDriver, VirtualPadDriver};
    use vigem_client::Error;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let open = args.iter().any(|a| a == "--open");
    let simulate = args
        .iter()
        .position(|a| a == "--simulate")
        .and_then(|i| args.get(i + 1));

    let status = match simulate.map(String::as_str) {
        None => VigemDriver::new().status(),
        Some("missing") => {
            setup::classify_bus_error(Error::BusNotFound, ServiceState::Missing, None)
        }
        Some("stopped") => {
            setup::classify_bus_error(Error::BusNotFound, ServiceState::Stopped, None)
        }
        Some("outdated") => setup::classify_bus_error(
            Error::BusVersionMismatch,
            ServiceState::Running,
            Some("1.9.0".to_owned()),
        ),
        Some(other) => {
            eprintln!("--simulate aceita missing, stopped ou outdated (recebido: {other})");
            std::process::exit(2);
        }
    };

    println!("serviço ViGEmBus : {:?}", setup::service_state());
    println!(
        "versão instalada : {}",
        setup::installed_version().unwrap_or_else(|| "(não encontrada)".to_owned())
    );
    println!("status           : {status:?}");

    match status {
        DriverStatus::Ready => println!("\nOK: o driver está pronto."),
        DriverStatus::NotInstalled | DriverStatus::Outdated { .. } => {
            println!("\nAVISO: o ViGEmBus não está instalado ou está desatualizado.");
            println!("Baixe em: {VIGEM_DOWNLOAD_URL}");
            if open {
                match setup::open_download_page() {
                    Ok(()) => println!("(abrindo no navegador...)"),
                    Err(e) => eprintln!("não consegui abrir o navegador: {e}"),
                }
            }
        }
        DriverStatus::ServiceStopped => {
            println!("\nAVISO: o ViGEmBus está instalado, mas o serviço está parado.");
            println!("Inicie-o (terminal de administrador): sc start ViGEmBus");
        }
        DriverStatus::Error(message) => println!("\nERRO: {message}"),
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("O ViGEmBus só existe no Windows.");
}
