//! Simulador de client do `PadGaming`: testa o server sem o app Android.
//!
//! - [`client`]: envia input a uma taxa fixa, Pings a cada 250 ms e mede o RTT.
//! - [`source`]: padrões prontos, scripts TOML e (com a feature `gamepad`) um controle físico.
//! - [`link`]: perda, atraso, jitter e reordenação simulados.
//! - [`report`]: relatório do `bench` em texto e JSON.
//! - [`cli`]: a linha de comando.

// Ferramenta de teste: as conversões entre inteiros e `f32`/`f64` (analógicos, percentis, RNG)
// trabalham com valores sempre limitados antes, e as comparações exatas de float dos testes
// são intencionais.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::float_cmp
)]

pub mod cli;
pub mod client;
pub mod link;
pub mod report;
pub mod source;
pub mod units;

/// `session_id` da sessão de desenvolvimento (`--insecure-dev`). Igual a
/// `padgaming_net::input::DEV_SESSION_ID`; o teste de integração do server confere.
pub const DEV_SESSION_ID: u32 = 1;
/// Chave da sessão de desenvolvimento. **Pública**: só serve para testes.
pub const DEV_KEY: [u8; padgaming_protocol::KEY_LEN] = [0x42; padgaming_protocol::KEY_LEN];
