//! Pareamento e descoberta: leitura da URI do QR e da resposta de broadcast.

use vynpad_protocol::discovery::{BROADCAST_PORT, BROADCAST_REQUEST, DiscoveryResponse};
use vynpad_protocol::pairing_uri::PairingUri;

use crate::CoreError;

/// Dados da URI `vynpad://pair?...` lida do QR.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PairingInfo {
    /// IPv4 do server em ordem de preferência (pelo menos um).
    pub hosts: Vec<String>,
    /// Porta TCP de controle.
    pub port: u16,
    /// SHA-256 do certificado do server (para o TLS pinning), 32 bytes.
    pub fingerprint: Vec<u8>,
    /// Token de pareamento de uso único, 16 bytes. É segredo: não registrar em log.
    pub pairing_token: Vec<u8>,
    /// Nome do server.
    pub name: String,
}

/// Lê a URI de pareamento do QR.
#[uniffi::export]
pub fn parse_pairing_uri(uri: &str) -> Result<PairingInfo, CoreError> {
    let parsed = PairingUri::parse(uri)?;
    Ok(PairingInfo {
        hosts: parsed.hosts.iter().map(ToString::to_string).collect(),
        port: parsed.port,
        fingerprint: parsed.fingerprint.to_vec(),
        pairing_token: parsed.pairing_token.expose().to_vec(),
        name: parsed.name,
    })
}

/// Um server que respondeu ao broadcast de descoberta.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DiscoveredServer {
    /// UUID do server.
    pub id: String,
    /// Nome exibido.
    pub name: String,
    /// `proto_ver` suportado.
    pub protocol_version: u8,
    /// Porta TCP de controle.
    pub tcp_port: u16,
    /// Porta UDP de input.
    pub udp_port: u16,
    /// `windows` ou `linux`.
    pub os: String,
    /// Slots livres (0–4).
    pub free_slots: u8,
    /// 8 primeiros bytes do fingerprint em hex (identificação visual, não substitui o pinning).
    pub short_fingerprint: String,
}

/// Lê (e valida) a resposta JSON de um server ao broadcast.
#[uniffi::export]
pub fn parse_discovery_response(json: &[u8]) -> Result<DiscoveredServer, CoreError> {
    let r = DiscoveryResponse::from_json(json).map_err(|e| CoreError::Discovery { reason: e.0 })?;
    Ok(DiscoveredServer {
        id: r.id.as_str().to_owned(),
        name: r.name,
        protocol_version: r.ver,
        tcp_port: r.tcp,
        udp_port: r.udp,
        os: r.os,
        free_slots: r.free,
        short_fingerprint: r.fp,
    })
}

/// Bytes do pedido de descoberta a enviar por broadcast UDP.
#[uniffi::export]
pub fn discovery_request() -> Vec<u8> {
    BROADCAST_REQUEST.to_vec()
}

/// Porta UDP do broadcast de descoberta.
#[uniffi::export]
pub fn discovery_port() -> u16 {
    BROADCAST_PORT
}
