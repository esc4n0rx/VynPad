//! Pacote de diagnóstico (T-705): um `.zip` com o que o suporte precisa e **nenhum segredo**.
//!
//! O que entra: informações do sistema, configurações, interfaces de rede (IPs mascarados se o
//! usuário quiser), métricas recentes, a lista de dispositivos (sem tokens) e os logs dos últimos
//! dias. Tudo passa por [`redact`], que tira tokens, chaves e PINs mesmo que algum tenha
//! escapado para um log. O arquivo é montado em memória a partir de dados que o chamador junta;
//! nada aqui lê o disco, o que o deixa testável.

use std::io::{Cursor, Write};

use serde::Serialize;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// O texto que substitui um segredo.
pub const REDACTED: &str = "[redigido]";

/// Nomes de campos cujo valor é sempre segredo (comparação sem diferenciar maiúsculas).
const SECRET_KEYS: [&str; 9] = [
    "token",
    "device_token",
    "udp_key",
    "secret",
    "password",
    "authorization",
    "pin",
    "token_hash",
    "key",
];

/// Palavras com pelo menos este tamanho e dígitos misturados a letras parecem token (base64 de
/// 16 bytes tem 22 caracteres).
const TOKEN_MIN_LEN: usize = 22;
/// Hex puro com pelo menos este tamanho parece hash ou chave (um fingerprint curto de 16 não;
/// um token de 12 bytes em hex tem 24).
const HEX_MIN_LEN: usize = 24;

/// Um arquivo de log a incluir.
#[derive(Debug, Clone)]
pub struct LogFile {
    /// Nome do arquivo (sem pasta).
    pub name: String,
    /// Conteúdo em texto.
    pub text: String,
}

/// Um dispositivo pareado, só com o que ajuda a diagnosticar (nunca token nem hash).
#[derive(Debug, Clone, Serialize)]
pub struct DeviceSummary {
    /// Identificador do dispositivo.
    pub id: String,
    /// Nome.
    pub name: String,
    /// Modelo.
    pub model: Option<String>,
    /// Versão do SO.
    pub os_version: Option<String>,
    /// Quando pareou (epoch ms).
    pub paired_at: i64,
    /// Última vez visto (epoch ms).
    pub last_seen_at: Option<i64>,
    /// Foi revogado.
    pub revoked: bool,
    /// Slot preferido.
    pub preferred_slot: Option<u8>,
}

/// Tudo o que vai para o pacote.
#[derive(Debug, Clone)]
pub struct Diagnostics {
    /// Versões, sistema, driver, portas (texto livre).
    pub info: String,
    /// Configurações atuais, já em JSON.
    pub settings_json: String,
    /// `(interface, ip)` das redes locais.
    pub interfaces: Vec<(String, String)>,
    /// Métricas recentes, já em JSON.
    pub metrics_json: String,
    /// Dispositivos pareados.
    pub devices: Vec<DeviceSummary>,
    /// Logs dos últimos dias.
    pub logs: Vec<LogFile>,
    /// Troca os dois últimos octetos dos IPv4 por `x`.
    pub mask_ips: bool,
}

/// Erro ao montar o pacote.
#[derive(Debug, thiserror::Error)]
pub enum DiagnosticsError {
    /// Falha ao escrever o `.zip`.
    #[error("não consegui montar o pacote de diagnóstico: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// Falha de escrita em memória.
    #[error("não consegui montar o pacote de diagnóstico: {0}")]
    Io(#[from] std::io::Error),
    /// Falha ao serializar a lista de dispositivos.
    #[error("não consegui montar o pacote de diagnóstico: {0}")]
    Json(#[from] serde_json::Error),
}

/// Monta o `.zip` (em memória) com todos os textos já redigidos.
pub fn build_zip(diagnostics: &Diagnostics) -> Result<Vec<u8>, DiagnosticsError> {
    let mask = diagnostics.mask_ips;
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut add = |name: &str, text: &str| -> Result<(), DiagnosticsError> {
        zip.start_file(name, options)?;
        zip.write_all(redact(text, mask).as_bytes())?;
        Ok(())
    };

    add("info.txt", &diagnostics.info)?;
    add("settings.json", &diagnostics.settings_json)?;
    add("metrics.json", &diagnostics.metrics_json)?;
    let interfaces = diagnostics
        .interfaces
        .iter()
        .map(|(name, ip)| format!("{name}: {ip}"))
        .collect::<Vec<_>>()
        .join("\n");
    add("network.txt", &interfaces)?;
    add(
        "devices.json",
        &serde_json::to_string_pretty(&diagnostics.devices)?,
    )?;
    for log in &diagnostics.logs {
        // O nome do arquivo vem do disco: só o último componente e sem barras.
        let name = log.name.rsplit(['/', '\\']).next().unwrap_or("log");
        add(&format!("logs/{name}"), &log.text)?;
    }
    Ok(zip.finish()?.into_inner())
}

/// Tira segredos do `text`: valores de campos como `token`, `udp_key` e `pin`, palavras que
/// parecem token (base64 longo) ou hash (hex longo). Com `mask_ips`, os IPv4 ficam `a.b.x.x`.
pub fn redact(text: &str, mask_ips: bool) -> String {
    let by_key = redact_by_key(text);
    let by_shape = redact_by_shape(&by_key);
    if mask_ips {
        mask_ipv4(&by_shape)
    } else {
        by_shape
    }
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+' | '/' | '=')
}

/// `token=abc`, `"udp_key": "abc"`, `pin: 123456`: troca só o valor.
fn redact_by_key(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        let matched = SECRET_KEYS.iter().find_map(|key| {
            // A chave precisa começar numa fronteira de palavra (`token_hash` não é `hash`).
            let starts = lower[i..].starts_with(key)
                && (i == 0 || !is_word_char(char::from(bytes[i - 1])) || bytes[i - 1] == b'_');
            starts.then_some(key.len())
        });
        if let Some(key_len) = matched {
            let mut j = i + key_len;
            // Fecha a aspa do nome e pula `:`/`=`, espaços e a aspa do valor.
            while j < text.len() && matches!(bytes[j], b'"' | b'\'') {
                j += 1;
            }
            let sep_start = j;
            while j < text.len() && matches!(bytes[j], b' ' | b':' | b'=') {
                j += 1;
            }
            let has_sep =
                j > sep_start && bytes[sep_start..j].iter().any(|b| matches!(b, b':' | b'='));
            while j < text.len() && matches!(bytes[j], b'"' | b'\'' | b' ') {
                j += 1;
            }
            let value_start = j;
            while j < text.len()
                && !matches!(
                    bytes[j],
                    b' ' | b'"' | b'\'' | b',' | b'}' | b']' | b'\n' | b'\r' | b';'
                )
            {
                j += 1;
            }
            if has_sep && j > value_start {
                out.push_str(&text[i..value_start]);
                out.push_str(REDACTED);
                i = j;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn is_uuid(word: &str) -> bool {
    let parts: Vec<&str> = word.split('-').collect();
    parts.len() == 5
        && parts
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(p, n)| p.len() == n && p.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn looks_secret(word: &str) -> bool {
    if is_uuid(word) {
        return false;
    }
    let len = word.len();
    let hex = word.bytes().all(|b| b.is_ascii_hexdigit());
    if hex {
        return len >= HEX_MIN_LEN;
    }
    let has_digit = word.bytes().any(|b| b.is_ascii_digit());
    let has_letter = word.bytes().any(|b| b.is_ascii_alphabetic());
    len >= TOKEN_MIN_LEN && has_digit && has_letter
}

/// Palavras com cara de segredo (base64 ou hex longos), em qualquer lugar do texto.
fn redact_by_shape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if !word.is_empty() {
            if looks_secret(word) {
                out.push_str(REDACTED);
            } else {
                out.push_str(word);
            }
            word.clear();
        }
    };
    for c in text.chars() {
        if is_word_char(c) {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

/// `192.168.0.10` → `192.168.x.x` (só IPv4 bem formado).
fn mask_ipv4(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit()
            && (i == 0 || !(chars[i - 1].is_ascii_digit() || chars[i - 1] == '.'))
        {
            // Tenta ler a.b.c.d a partir daqui.
            let mut j = i;
            let mut octets = Vec::new();
            loop {
                let start = j;
                while j < chars.len() && chars[j].is_ascii_digit() && j - start < 3 {
                    j += 1;
                }
                if j == start {
                    break;
                }
                let value: String = chars[start..j].iter().collect();
                octets.push(value);
                if octets.len() == 4 {
                    break;
                }
                if j < chars.len() && chars[j] == '.' {
                    j += 1;
                } else {
                    break;
                }
            }
            let valid = octets.len() == 4
                && octets
                    .iter()
                    .all(|o| o.parse::<u16>().is_ok_and(|n| n <= 255))
                && !(j < chars.len()
                    && (chars[j].is_ascii_digit()
                        || chars[j] == '.' && chars.get(j + 1).is_some_and(char::is_ascii_digit)));
            if valid {
                out.push_str(&octets[0]);
                out.push('.');
                out.push_str(&octets[1]);
                out.push_str(".x.x");
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// `YYYYMMDD-HHMMSS` em UTC, para o nome do arquivo.
pub fn utc_stamp(epoch_ms: i64) -> String {
    let secs = epoch_ms.div_euclid(1000);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Dias desde 1970-01-01 → data civil (algoritmo de Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    const B64_TOKEN: &str = "Zm9vYmFyYmF6cXV4MTIzNDU2Nzg";
    const HEX_KEY: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
    const UUID: &str = "5b0c8a1e-2f4d-4c7a-9e1b-0d3f6a7b8c9d";

    fn text_of(zip_bytes: &[u8], name: &str) -> String {
        let mut zip = zip::ZipArchive::new(Cursor::new(zip_bytes)).unwrap();
        let mut file = zip.by_name(name).unwrap();
        let mut text = String::new();
        file.read_to_string(&mut text).unwrap();
        text
    }

    fn all_text(zip_bytes: &[u8]) -> String {
        let mut zip = zip::ZipArchive::new(Cursor::new(zip_bytes)).unwrap();
        let mut all = String::new();
        for i in 0..zip.len() {
            let mut file = zip.by_index(i).unwrap();
            let mut text = String::new();
            file.read_to_string(&mut text).unwrap();
            all.push_str(file.name());
            all.push('\n');
            all.push_str(&text);
            all.push('\n');
        }
        all
    }

    fn sample(mask: bool) -> Diagnostics {
        Diagnostics {
            info: format!(
                "VynPad 0.1.0 (Windows)\nfingerprint: 3fa1c2d4e5f60718\nserver_id: {UUID}"
            ),
            settings_json: r#"{"language":"pt-BR","server_name":"PC da sala"}"#.to_owned(),
            interfaces: vec![("Wi-Fi".to_owned(), "192.168.0.10".to_owned())],
            metrics_json: r#"{"slots":[]}"#.to_owned(),
            devices: vec![DeviceSummary {
                id: UUID.to_owned(),
                name: "Galaxy da Ana".to_owned(),
                model: Some("SM-S918B".to_owned()),
                os_version: Some("Android 15".to_owned()),
                paired_at: 1,
                last_seen_at: Some(2),
                revoked: false,
                preferred_slot: None,
            }],
            logs: vec![LogFile {
                name: "C:\\Users\\ana\\logs\\vynpad.2026-10-03.log".to_owned(),
                text: format!(
                    "INFO conectado device_token={B64_TOKEN} de 192.168.0.77\n\
                     INFO welcome udp_key: \"{HEX_KEY}\" session=7\n\
                     WARN pin=482913 recusado\n\
                     DEBUG {{\"token\":\"{B64_TOKEN}\",\"pin\":\"482913\"}}\n\
                     INFO dispositivo {UUID} fingerprint 3fa1c2d4e5f60718\n\
                     INFO palavra comum e Authorization: Bearer abcdef1234567890ABCDEF12\n"
                ),
            }],
            mask_ips: mask,
        }
    }

    #[test]
    fn no_secret_survives_in_the_zip() {
        let bytes = build_zip(&sample(false)).unwrap();
        let all = all_text(&bytes);
        for secret in [B64_TOKEN, HEX_KEY, "482913", "abcdef1234567890ABCDEF12"] {
            assert!(!all.contains(secret), "o segredo {secret} vazou");
        }
        assert!(all.contains(REDACTED));
        // O que ajuda o suporte continua: o id do dispositivo, o fingerprint curto e as frases.
        assert!(all.contains(UUID));
        assert!(all.contains("3fa1c2d4e5f60718"));
        assert!(all.contains("INFO palavra comum"));
        assert!(all.contains("Galaxy da Ana"));
        assert!(!all.contains("token_hash"));
    }

    #[test]
    fn the_zip_has_the_expected_files_and_flattens_log_paths() {
        let bytes = build_zip(&sample(false)).unwrap();
        let mut zip = zip::ZipArchive::new(Cursor::new(&bytes[..])).unwrap();
        let mut names: Vec<_> = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_owned())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "devices.json",
                "info.txt",
                "logs/vynpad.2026-10-03.log",
                "metrics.json",
                "network.txt",
                "settings.json"
            ]
        );
    }

    #[test]
    fn ips_are_masked_only_when_asked() {
        let plain = text_of(&build_zip(&sample(false)).unwrap(), "network.txt");
        assert!(plain.contains("192.168.0.10"));
        let masked_zip = build_zip(&sample(true)).unwrap();
        let all = all_text(&masked_zip);
        assert!(!all.contains("192.168.0.10"));
        assert!(!all.contains("192.168.0.77"));
        assert!(text_of(&masked_zip, "network.txt").contains("192.168.x.x"));
    }

    #[test]
    fn redact_handles_each_shape() {
        assert_eq!(redact("token=abc123", false), format!("token={REDACTED}"));
        assert_eq!(
            redact(r#"{"udp_key": "AAAA1111"}"#, false),
            format!(r#"{{"udp_key": "{REDACTED}"}}"#)
        );
        assert_eq!(
            redact("pin: 123456 ok", false),
            format!("pin: {REDACTED} ok")
        );
        assert_eq!(
            redact(&format!("x {B64_TOKEN} y"), false),
            format!("x {REDACTED} y")
        );
        assert_eq!(
            redact(&format!("h {HEX_KEY}."), false),
            format!("h {REDACTED}.")
        );
        // Sem separador, "pin" é só uma palavra (pinning, spinner...).
        assert_eq!(
            redact("pinning do certificado", false),
            "pinning do certificado"
        );
        assert_eq!(redact("o token expirou", false), "o token expirou");
    }

    #[test]
    fn ordinary_text_is_left_alone() {
        let line = "2026-10-03T12:00:00Z INFO vynpad_net::control::server: servidor de controle TLS iniciado local_addr=0.0.0.0:47800";
        assert_eq!(redact(line, false), line);
        assert_eq!(redact(UUID, false), UUID);
        assert_eq!(redact("3fa1c2d4e5f60718", false), "3fa1c2d4e5f60718");
        assert_eq!(redact("versão 1.2.3.4.5", true), "versão 1.2.3.4.5");
    }

    #[test]
    fn ipv4_masking_is_precise() {
        assert_eq!(
            mask_ipv4("de 10.0.1.25 para 192.168.0.1:47800"),
            "de 10.0.x.x para 192.168.x.x:47800"
        );
        assert_eq!(mask_ipv4("300.1.1.1"), "300.1.1.1");
        assert_eq!(mask_ipv4("1.2.3"), "1.2.3");
        assert_eq!(mask_ipv4("sem ip"), "sem ip");
    }

    #[test]
    fn the_file_name_stamp_is_utc() {
        assert_eq!(utc_stamp(0), "19700101-000000");
        assert_eq!(utc_stamp(1_759_492_800_000), "20251003-120000");
        assert_eq!(utc_stamp(951_782_400_000), "20000229-000000");
    }
}
