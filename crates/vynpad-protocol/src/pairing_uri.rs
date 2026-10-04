//! URI do QR code de pareamento (PROTOCOLO §4.2): `vynpad://pair?v=1&h=...&p=...`.

use core::fmt::{self, Write as _};
use std::net::Ipv4Addr;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::secret::Secret;

/// Versão da URI.
pub const URI_VERSION: u32 = 1;
/// Maior nome de server aceito, em caracteres.
pub const MAX_NAME_CHARS: usize = 64;

const PREFIX: &str = "vynpad://pair";
/// Parâmetros obrigatórios, na ordem em que `parse` os desestrutura.
const KEYS: [&str; 6] = ["v", "h", "p", "fp", "t", "n"];

/// Erro de parsing; os códigos são os dos test vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingUriError {
    /// Esquema diferente de `vynpad` ou ação diferente de `pair`.
    BadScheme,
    /// Falta um parâmetro obrigatório.
    MissingParam,
    /// Parâmetro repetido, malformado ou fora da faixa.
    BadParam,
    /// `v` diferente de 1.
    UnsupportedVersion,
}

impl PairingUriError {
    /// Código estável (`bad_scheme`, `missing_param`, `bad_param`, `unsupported_version`).
    pub const fn code(self) -> &'static str {
        match self {
            Self::BadScheme => "bad_scheme",
            Self::MissingParam => "missing_param",
            Self::BadParam => "bad_param",
            Self::UnsupportedVersion => "unsupported_version",
        }
    }
}

impl fmt::Display for PairingUriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for PairingUriError {}

/// Conteúdo da URI de pareamento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingUri {
    /// IPv4 do server, em ordem de preferência (pelo menos um).
    pub hosts: Vec<Ipv4Addr>,
    /// Porta TCP de controle.
    pub port: u16,
    /// SHA-256 do certificado do server.
    pub fingerprint: [u8; 32],
    /// Token de pareamento de uso único.
    pub pairing_token: Secret<[u8; 16]>,
    /// Nome do server (1–64 caracteres).
    pub name: String,
}

impl PairingUri {
    /// Monta a URI. Falha se faltar host ou se o nome não tiver 1–64 caracteres.
    pub fn encode(&self) -> Result<String, PairingUriError> {
        let name_len = self.name.chars().count();
        if self.hosts.is_empty() || name_len == 0 || name_len > MAX_NAME_CHARS {
            return Err(PairingUriError::BadParam);
        }
        let hosts: Vec<String> = self.hosts.iter().map(ToString::to_string).collect();
        Ok(format!(
            "{PREFIX}?v={URI_VERSION}&h={}&p={}&fp={}&t={}&n={}",
            hosts.join(","),
            self.port,
            URL_SAFE_NO_PAD.encode(self.fingerprint),
            URL_SAFE_NO_PAD.encode(self.pairing_token.expose()),
            percent_encode(&self.name),
        ))
    }

    /// Lê uma URI. Parâmetros desconhecidos são ignorados; repetidos são erro.
    pub fn parse(uri: &str) -> Result<Self, PairingUriError> {
        let (head, query) = uri.split_once('?').ok_or(PairingUriError::BadScheme)?;
        if head != PREFIX {
            return Err(PairingUriError::BadScheme);
        }

        let mut found: [Option<String>; 6] = Default::default();
        for pair in query.split('&').filter(|s| !s.is_empty()) {
            let (key, raw) = pair.split_once('=').ok_or(PairingUriError::BadParam)?;
            let Some(index) = KEYS.iter().position(|k| *k == key) else {
                continue;
            };
            if found[index].replace(percent_decode(raw)?).is_some() {
                return Err(PairingUriError::BadParam);
            }
        }
        let [version, hosts, port, fingerprint, token, name] = found;
        let need = |o: Option<String>| o.ok_or(PairingUriError::MissingParam);
        let (version, hosts, port) = (need(version)?, need(hosts)?, need(port)?);
        let (fingerprint, token, name) = (need(fingerprint)?, need(token)?, need(name)?);

        match version.parse::<u32>() {
            Ok(URI_VERSION) => {}
            Ok(_) => return Err(PairingUriError::UnsupportedVersion),
            Err(_) => return Err(PairingUriError::BadParam),
        }
        let hosts = hosts
            .split(',')
            .map(|s| s.parse::<Ipv4Addr>().map_err(|_| PairingUriError::BadParam))
            .collect::<Result<Vec<_>, _>>()?;
        let port = port
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .ok_or(PairingUriError::BadParam)?;
        let name_len = name.chars().count();
        if name_len == 0 || name_len > MAX_NAME_CHARS {
            return Err(PairingUriError::BadParam);
        }
        Ok(Self {
            hosts,
            port,
            fingerprint: decode_b64(&fingerprint)?,
            pairing_token: Secret::new(decode_b64(&token)?),
            name,
        })
    }
}

fn decode_b64<const N: usize>(text: &str) -> Result<[u8; N], PairingUriError> {
    URL_SAFE_NO_PAD
        .decode(text.as_bytes())
        .ok()
        .and_then(|raw| raw.try_into().ok())
        .ok_or(PairingUriError::BadParam)
}

/// Percent-encoding UTF-8; só os não reservados ficam literais e o espaço vira `%20`.
fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// Decodifica `%XX` sem tratar `+` como espaço.
fn percent_decode(text: &str) -> Result<String, PairingUriError> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3).ok_or(PairingUriError::BadParam)?;
            let hex = core::str::from_utf8(hex).map_err(|_| PairingUriError::BadParam)?;
            out.push(u8::from_str_radix(hex, 16).map_err(|_| PairingUriError::BadParam)?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| PairingUriError::BadParam)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PairingUri {
        PairingUri {
            hosts: vec![Ipv4Addr::new(192, 168, 0, 10), Ipv4Addr::new(10, 0, 0, 5)],
            port: 47800,
            fingerprint: [7; 32],
            pairing_token: Secret::new([9; 16]),
            name: "Sala de Jogos — João+1".into(),
        }
    }

    #[test]
    fn roundtrip_with_utf8_and_plus() {
        let uri = sample().encode().unwrap();
        assert!(uri.contains("n=Sala%20de%20Jogos%20%E2%80%94%20Jo%C3%A3o%2B1"));
        assert!(!uri.contains('+'));
        assert_eq!(PairingUri::parse(&uri).unwrap(), sample());
    }

    #[test]
    fn encode_rejects_invalid_input() {
        let mut empty_name = sample();
        empty_name.name.clear();
        assert_eq!(empty_name.encode(), Err(PairingUriError::BadParam));
        let mut no_hosts = sample();
        no_hosts.hosts.clear();
        assert_eq!(no_hosts.encode(), Err(PairingUriError::BadParam));
    }
}
