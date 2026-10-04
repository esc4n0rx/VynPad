//! Logging do app com `tracing`: arquivo com rotação diária, saída no terminal e hook de pânico.
//!
//! Logs do crate `log` (usado pelo Tauri e por outras dependências) também chegam aqui,
//! via a ponte `tracing-log` que o `tracing-subscriber` instala no `init()`.

use std::path::PathBuf;

use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// `info` para o nosso código e para o frontend, `warn` para as dependências.
/// A variável `RUST_LOG` sobrescreve.
const DEFAULT_FILTER: &str = "warn,vynpad=info,frontend=info";

/// Quantidade de arquivos diários mantidos na pasta de logs.
const LOG_FILES_KEPT: usize = 7;

/// Pasta de logs documentada em ARQUITETURA §6.
pub fn log_dir() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    let dir = dirs::state_dir().map(|d| d.join("vynpad").join("logs"));
    #[cfg(not(target_os = "linux"))]
    let dir = dirs::data_local_dir().map(|d| d.join("VynPad").join("logs"));
    dir
}

/// Inicializa o logging. Deve ser chamada uma única vez, antes de criar o app Tauri.
///
/// Se a pasta de logs não puder ser criada, o app segue só com a saída no terminal.
pub fn init() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));

    // Escrita síncrona de propósito: o volume é baixo (o hot path não loga por pacote)
    // e garante que a mensagem de pânico chegue ao disco antes do `panic = "abort"`.
    let file_layer = log_dir().and_then(|dir| match file_appender(&dir) {
        Ok(appender) => Some(fmt::layer().with_ansi(false).with_writer(appender)),
        Err(err) => {
            eprintln!("não foi possível abrir os logs em {}: {err}", dir.display());
            None
        }
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(fmt::layer().with_ansi(cfg!(debug_assertions)))
        .init();

    install_panic_hook();
}

fn file_appender(dir: &std::path::Path) -> Result<RollingFileAppender, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    Ok(RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("vynpad")
        .filename_suffix("log")
        .max_log_files(LOG_FILES_KEPT)
        .build(dir)?)
}

/// Registra o pânico (com backtrace) no log antes de delegar ao hook padrão.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        tracing::error!(target: "vynpad::panic", %info, %backtrace, "pânico");
        default_hook(info);
    }));
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use super::*;

    /// Writer em memória para inspecionar o que foi logado.
    #[derive(Clone, Default)]
    struct Captura(Arc<Mutex<Vec<u8>>>);

    impl Write for Captura {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn panico_e_registrado_no_log() {
        let captura = Captura::default();
        let writer = captura.clone();
        let subscriber = tracing_subscriber::registry().with(
            fmt::layer()
                .with_ansi(false)
                .with_writer(move || writer.clone()),
        );

        install_panic_hook();
        let resultado = tracing::subscriber::with_default(subscriber, || {
            std::panic::catch_unwind(|| panic!("falha proposital do teste"))
        });

        assert!(resultado.is_err());
        let log = String::from_utf8(captura.0.lock().unwrap().clone()).unwrap();
        assert!(log.contains("pânico"), "log: {log}");
        assert!(log.contains("falha proposital do teste"), "log: {log}");
    }
}
