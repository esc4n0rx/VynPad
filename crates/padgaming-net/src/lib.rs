//! Rede do server: loop UDP de input, canal de controle TLS e descoberta mDNS.
//!
//! Por enquanto expõe a base de segurança (T-103):
//! - [`identity`]: certificado autoassinado, fingerprint e configuração TLS 1.3.
//! - [`pairing`]: tokens, PIN, rate limit, device tokens e a trait [`pairing::PairingService`].
//! - [`qr`]: QR code (SVG) da URI de pareamento.

pub mod identity;
pub mod pairing;
pub mod qr;

/// `N` bytes aleatórios do gerador do sistema operacional.
pub(crate) fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).expect("gerador aleatório do sistema indisponível");
    bytes
}
