//! O que o server anuncia sobre si mesmo (PROTOCOLO §3).

use padgaming_protocol::control::Uuid;
use padgaming_protocol::discovery::{DiscoveryResponse, MAX_NAME_LEN, short_fingerprint, txt};

/// Dados anunciados por mDNS e por broadcast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerInfo {
    /// Identificador estável do server.
    pub id: Uuid,
    /// Nome exibido.
    pub name: String,
    /// Porta TCP de controle.
    pub tcp_port: u16,
    /// Porta UDP de input.
    pub udp_port: u16,
    /// `windows` ou `linux`.
    pub os: String,
    /// Slots livres (0–4).
    pub free: u8,
    /// Fingerprint do certificado.
    pub fingerprint: [u8; 32],
}

impl ServerInfo {
    /// O SO em que o server está rodando.
    pub const fn current_os() -> &'static str {
        std::env::consts::OS
    }

    /// O nome como vai no fio: pelo menos 1 caractere e no máximo 63 (corta em fronteira de
    /// caractere).
    pub fn wire_name(&self) -> String {
        let trimmed: String = self.name.trim().chars().take(MAX_NAME_LEN).collect();
        if trimmed.is_empty() {
            "PadGaming".to_owned()
        } else {
            trimmed
        }
    }

    /// A resposta ao broadcast `PGDISC1`.
    pub fn response(&self) -> DiscoveryResponse {
        DiscoveryResponse {
            id: self.id.clone(),
            name: self.wire_name(),
            ver: 1,
            tcp: self.tcp_port,
            udp: self.udp_port,
            os: self.os.clone(),
            free: self.free.min(4),
            fp: short_fingerprint(&self.fingerprint),
        }
    }

    /// O TXT do anúncio mDNS.
    pub fn txt(&self) -> Vec<(&'static str, String)> {
        vec![
            (txt::ID, self.id.as_str().to_owned()),
            (txt::NAME, self.wire_name()),
            (txt::VER, "1".to_owned()),
            (txt::UDP, self.udp_port.to_string()),
            (txt::OS, self.os.clone()),
            (txt::FREE, self.free.min(4).to_string()),
            (txt::FP, short_fingerprint(&self.fingerprint)),
        ]
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn sample() -> ServerInfo {
        ServerInfo {
            id: Uuid::parse("5b0c8a1e-2f4d-4c7a-9e1b-0d3f6a7b8c9d").unwrap(),
            name: "PC-Sala".into(),
            tcp_port: 47800,
            udp_port: 47801,
            os: "windows".into(),
            free: 3,
            fingerprint: [0xab; 32],
        }
    }

    #[test]
    fn txt_follows_the_protocol_keys() {
        let txt = sample().txt();
        let get = |k: &str| {
            txt.iter()
                .find(|(key, _)| *key == k)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(get("id"), Some("5b0c8a1e-2f4d-4c7a-9e1b-0d3f6a7b8c9d"));
        assert_eq!(get("name"), Some("PC-Sala"));
        assert_eq!(get("ver"), Some("1"));
        assert_eq!(get("udp"), Some("47801"));
        assert_eq!(get("os"), Some("windows"));
        assert_eq!(get("free"), Some("3"));
        assert_eq!(get("fp"), Some("abababababababab"));
    }

    #[test]
    fn the_response_is_valid_for_the_schema() {
        let response = sample().response();
        assert!(response.validate().is_ok());
        assert_eq!(
            (response.tcp, response.udp, response.free),
            (47800, 47801, 3)
        );
    }

    #[test]
    fn names_are_trimmed_cut_at_63_characters_and_never_empty() {
        let mut info = sample();
        info.name = "é".repeat(100);
        assert_eq!(info.wire_name().chars().count(), 63);
        assert!(info.response().validate().is_ok());
        info.name = "   ".into();
        assert_eq!(info.wire_name(), "PadGaming");
    }

    #[test]
    fn free_is_capped_at_four() {
        let mut info = sample();
        info.free = 9;
        assert_eq!(info.response().free, 4);
    }
}
