//! Client simulador em CLI para testar o server sem o app Android.

use clap::Parser as _;
use padgaming_sim::cli::{Cli, run};

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("erro: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
