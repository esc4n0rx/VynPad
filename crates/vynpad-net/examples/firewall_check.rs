//! Confere o firewall do Windows para um executável e, com `--fix`, cria as regras (abre o UAC).
//!
//! ```text
//! cargo run -p vynpad-net --example firewall_check                  # só lê (não precisa de admin)
//! cargo run -p vynpad-net --example firewall_check -- --exe C:\caminho\vynpad.exe
//! cargo run -p vynpad-net --example firewall_check -- --fix         # cria as regras (UAC)
//! ```
//!
//! Por padrão avalia este próprio exemplo; use `--exe` para o executável do app.

use std::path::PathBuf;

use vynpad_net::firewall::{self, FirewallPorts, FirewallStatus};

fn print(status: &FirewallStatus) {
    println!(
        "firewall ligado nos perfis ativos: {} · perfis ativos: {:?}",
        status.firewall_on, status.profiles
    );
    for port in &status.ports {
        let r = &port.requirement;
        println!(
            "  {:?} {:<5} {:<11} {}",
            r.transport,
            r.port,
            r.purpose,
            if port.allowed {
                "liberada"
            } else {
                "BLOQUEADA"
            }
        );
    }
    println!(
        "resultado: {}",
        if status.ok() { "ok" } else { "faltam regras" }
    );
    if status.public_network_warning() {
        println!(
            "AVISO: rede pública. Mude para privada em {}",
            firewall::NETWORK_SETTINGS_URI
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut exe = firewall::current_exe()?;
    let mut fix = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fix" => fix = true,
            "--exe" => exe = PathBuf::from(args.next().ok_or("--exe precisa de um caminho")?),
            other => return Err(format!("argumento desconhecido: {other}").into()),
        }
    }
    let ports = FirewallPorts::default();
    println!("executável: {}", exe.display());
    print(&firewall::check(&exe, ports)?);
    if fix {
        println!("criando as regras (aceite o pedido do UAC)...");
        print(&firewall::fix(&exe, ports)?);
    }
    Ok(())
}
