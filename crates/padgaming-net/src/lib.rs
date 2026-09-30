//! Rede do server: loop UDP de input, canal de controle TLS e descoberta mDNS.
//!
//! - [`input`]: loop UDP de input, o hot path (T-301).
//! - [`control`]: servidor de controle TLS: handshake, sessões e keepalive (T-302).
//!
//! Base de segurança (T-103):
//! - [`identity`]: certificado autoassinado, fingerprint e configuração TLS 1.3.
//! - [`pairing`]: tokens, PIN, rate limit, device tokens e a trait [`pairing::PairingService`].
//! - [`qr`]: QR code (SVG) da URI de pareamento.

pub mod control;
pub mod identity;
pub mod input;
pub mod pairing;
pub mod qr;

/// `N` bytes aleatórios do gerador do sistema operacional.
pub(crate) fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).expect("gerador aleatório do sistema indisponível");
    bytes
}
