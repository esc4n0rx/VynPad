//! Descoberta (PROTOCOLO §3): nomes dos serviços mDNS, chaves do TXT e a resposta ao
//! broadcast `VPDISC1`. O schema normativo da resposta é
//! `protocol/schemas/discovery-response.schema.json`.

use serde::{Deserialize, Serialize};

use crate::control::Uuid;

/// Serviço mDNS anunciado pelo server (porta TCP de controle).
pub const SERVER_SERVICE: &str = "_vynpad._tcp.local.";
/// Serviço mDNS anunciado pelo client enquanto está na tela de conexão.
pub const CLIENT_SERVICE: &str = "_vynpad-client._tcp.local.";
/// Datagrama de pedido do broadcast de fallback.
pub const BROADCAST_REQUEST: &[u8; 7] = b"VPDISC1";
/// Porta UDP do broadcast de fallback.
pub const BROADCAST_PORT: u16 = 47802;
/// Maior tamanho de nome (mDNS: rótulo de 63 bytes).
pub const MAX_NAME_LEN: usize = 63;

/// Chaves do TXT do server (§3.1).
pub mod txt {
    /// UUID do server.
    pub const ID: &str = "id";
    /// Nome exibido.
    pub const NAME: &str = "name";
    /// `proto_ver` suportado.
    pub const VER: &str = "ver";
    /// Porta UDP de input.
    pub const UDP: &str = "udp";
    /// Sistema operacional.
    pub const OS: &str = "os";
    /// Slots livres.
    pub const FREE: &str = "free";
    /// 8 primeiros bytes do fingerprint, em hex.
    pub const FP: &str = "fp";
    /// Modelo do aparelho (só no TXT do client).
    pub const MODEL: &str = "model";
}

/// Resposta do server ao broadcast (JSON sem framing, §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryResponse {
    /// UUID do server.
    pub id: Uuid,
    /// Nome exibido (1–63).
    pub name: String,
    /// `proto_ver` suportado.
    pub ver: u8,
    /// Porta TCP de controle.
    pub tcp: u16,
    /// Porta UDP de input.
    pub udp: u16,
    /// `windows` ou `linux`.
    pub os: String,
    /// Slots livres (0–4).
    pub free: u8,
    /// 8 primeiros bytes do fingerprint, em hex minúsculo (16 caracteres).
    pub fp: String,
}

/// Resposta inválida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadResponse(pub String);

impl core::fmt::Display for BadResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "resposta de descoberta inválida: {}", self.0)
    }
}

impl std::error::Error for BadResponse {}

impl DiscoveryResponse {
    /// Aplica as regras do schema que o serde não cobre.
    pub fn validate(&self) -> Result<(), BadResponse> {
        let bad = |why: &str| Err(BadResponse(why.to_owned()));
        let name_len = self.name.chars().count();
        if !(1..=MAX_NAME_LEN).contains(&name_len) {
            return bad("name fora de 1..=63");
        }
        if self.ver == 0 {
            return bad("ver = 0");
        }
        if self.tcp == 0 || self.udp == 0 {
            return bad("porta 0");
        }
        if self.free > 4 {
            return bad("free > 4");
        }
        if self.fp.len() != 16
            || !self
                .fp
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        {
            return bad("fp precisa de 16 caracteres hex minúsculos");
        }
        Ok(())
    }

    /// Serializa em JSON (sem framing).
    ///
    /// # Panics
    /// Nunca: a estrutura só tem tipos serializáveis.
    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("DiscoveryResponse é serializável")
    }

    /// Lê e valida uma resposta. Campos desconhecidos são ignorados.
    pub fn from_json(bytes: &[u8]) -> Result<Self, BadResponse> {
        let response: Self =
            serde_json::from_slice(bytes).map_err(|e| BadResponse(e.to_string()))?;
        response.validate()?;
        Ok(response)
    }
}

/// Os 8 primeiros bytes do fingerprint em hex minúsculo (o `fp` do TXT e do broadcast).
pub fn short_fingerprint(fingerprint: &[u8; 32]) -> String {
    use core::fmt::Write as _;
    let mut out = String::with_capacity(16);
    for byte in &fingerprint[..8] {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DiscoveryResponse {
        DiscoveryResponse {
            id: Uuid::parse("5b0c8a1e-2f4d-4c7a-9e1b-0d3f6a7b8c9d").unwrap(),
            name: "PC-Sala".into(),
            ver: 1,
            tcp: 47800,
            udp: 47801,
            os: "windows".into(),
            free: 3,
            fp: "3fa1c2d4e5f60718".into(),
        }
    }

    #[test]
    fn the_documented_example_parses_and_round_trips() {
        let json = br#"{"id":"5b0c8a1e-2f4d-4c7a-9e1b-0d3f6a7b8c9d","name":"PC-Sala","ver":1,"tcp":47800,"udp":47801,"os":"windows","free":3,"fp":"3fa1c2d4e5f60718"}"#;
        let parsed = DiscoveryResponse::from_json(json).unwrap();
        assert_eq!(parsed, sample());
        assert_eq!(
            DiscoveryResponse::from_json(&parsed.to_json()).unwrap(),
            parsed
        );
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let mut value: serde_json::Value = serde_json::from_slice(&sample().to_json()).unwrap();
        value["novo"] = serde_json::json!(true);
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(DiscoveryResponse::from_json(&bytes).unwrap(), sample());
    }

    #[test]
    fn schema_limits_are_enforced() {
        let mut r = sample();
        r.free = 5;
        assert!(r.validate().is_err());
        let mut r = sample();
        r.fp = "3FA1C2D4E5F60718".into();
        assert!(r.validate().is_err());
        let mut r = sample();
        r.name = String::new();
        assert!(r.validate().is_err());
        let mut r = sample();
        r.name = "x".repeat(64);
        assert!(r.validate().is_err());
        let mut r = sample();
        r.udp = 0;
        assert!(r.validate().is_err());
        assert!(DiscoveryResponse::from_json(b"{}").is_err());
        assert!(DiscoveryResponse::from_json(b"VPDISC1").is_err());
    }

    #[test]
    fn short_fingerprint_is_the_first_eight_bytes_in_hex() {
        let mut fp = [0u8; 32];
        fp[..8].copy_from_slice(&[0x3f, 0xa1, 0xc2, 0xd4, 0xe5, 0xf6, 0x07, 0x18]);
        fp[8] = 0xff;
        assert_eq!(short_fingerprint(&fp), "3fa1c2d4e5f60718");
    }
}
