//! Canal de controle (PROTOCOLO §5): mensagens JSON com framing `u32 LE`.
//!
//! O schema normativo é `protocol/schemas/control-message.schema.json`; os tipos daqui
//! aplicam as mesmas regras (serde para a forma, [`ControlMessage::validate`] para
//! faixas e tamanhos).

use core::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::secret::Secret;

/// Maior corpo de frame aceito, em bytes.
pub const MAX_FRAME_LEN: usize = 16384;
/// Tamanho do prefixo de tamanho do frame.
pub const FRAME_PREFIX_LEN: usize = 4;

/// Erro ao ler ou montar um frame de controle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlError {
    /// Tamanho do frame fora de 1..=[`MAX_FRAME_LEN`]; a conexão deve ser fechada.
    BadFrame,
    /// JSON inválido, que não é objeto ou que viola o schema. O texto diz o motivo.
    BadMessage(String),
    /// O frame ainda não chegou inteiro.
    Incomplete,
}

impl ControlError {
    /// Código estável (`bad_frame`, `bad_message`, `incomplete`), igual ao dos vectors.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::BadFrame => "bad_frame",
            Self::BadMessage(_) => "bad_message",
            Self::Incomplete => "incomplete",
        }
    }
}

impl fmt::Display for ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMessage(why) => write!(f, "bad_message: {why}"),
            other => f.write_str(other.code()),
        }
    }
}

impl std::error::Error for ControlError {}

fn bad(why: impl Into<String>) -> ControlError {
    ControlError::BadMessage(why.into())
}

// ---------------------------------------------------------------- Tipos auxiliares

/// `N` bytes transportados em base64url sem padding (forma canônica).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bytes<const N: usize>(pub [u8; N]);

impl<const N: usize> fmt::Debug for Bytes<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bytes<{N}>(")?;
        for b in &self.0 {
            write!(f, "{b:02x}")?;
        }
        f.write_str(")")
    }
}

impl<const N: usize> Serialize for Bytes<N> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&URL_SAFE_NO_PAD.encode(self.0))
    }
}

impl<'de, const N: usize> Deserialize<'de> for Bytes<N> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        // O engine padrão recusa padding e bits finais não canônicos.
        let raw = URL_SAFE_NO_PAD
            .decode(text.as_bytes())
            .map_err(|_| serde::de::Error::custom("base64url inválido"))?;
        let bytes: [u8; N] = raw
            .try_into()
            .map_err(|_| serde::de::Error::custom(format_args!("esperados {N} bytes")))?;
        Ok(Self(bytes))
    }
}

impl<T: Serialize> Serialize for Secret<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.expose().serialize(s)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Secret<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::deserialize(d).map(Secret::new)
    }
}

/// UUID em texto, minúsculo com hífens (`device_id`, `server_id`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Uuid(String);

impl Uuid {
    /// Valida o formato `8-4-4-4-12` em hex minúsculo.
    pub fn parse(text: &str) -> Result<Self, ControlError> {
        let groups: Vec<&str> = text.split('-').collect();
        let ok = groups.len() == 5
            && groups.iter().zip([8, 4, 4, 4, 12]).all(|(g, len)| {
                g.len() == len && g.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
            });
        if ok {
            Ok(Self(text.to_owned()))
        } else {
            Err(bad("UUID inválido"))
        }
    }

    /// O UUID em texto.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Uuid {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------- Mensagens

/// Mensagem do canal de controle. O discriminador no JSON é `"type"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlMessage {
    /// C→S: primeira mensagem do client.
    Hello(Hello),
    /// S→C: sessão criada.
    Welcome(Welcome),
    /// S→C: conexão recusada (o server fecha em seguida).
    Reject(Reject),
    /// S→C: configuração atual (sempre completa).
    Config(Config),
    /// Ambos: encerramento limpo.
    Bye(Bye),
    /// Ambos: keepalive.
    Ping,
    /// Ambos: resposta ao keepalive.
    Pong,
}

/// Valores de `type` conhecidos na v1; os demais são ignorados.
const KNOWN_TYPES: [&str; 7] = [
    "hello", "welcome", "reject", "config", "bye", "ping", "pong",
];

/// `hello` (C→S).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    /// Maior versão do protocolo que o client conhece.
    pub proto_ver: u8,
    /// Identificador estável do aparelho.
    pub device_id: Uuid,
    /// Nome exibido (1–64 caracteres).
    pub name: String,
    /// Modelo do aparelho (≤ 64).
    pub model: String,
    /// Versão do SO (≤ 32).
    pub os_ver: String,
    /// Versão do app (≤ 32).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_ver: Option<String>,
    /// Recursos do aparelho.
    pub caps: Caps,
    /// Credencial.
    pub auth: Auth,
}

/// Recursos do client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Caps {
    /// Tem giroscópio/acelerômetro.
    pub gyro: bool,
    /// Consegue vibrar.
    pub rumble: bool,
}

/// Credencial do `hello`; o discriminador no JSON é `"method"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum Auth {
    /// Token de uso único do QR code (16 bytes).
    PairingToken {
        /// O token.
        token: Secret<Bytes<16>>,
    },
    /// PIN de 6 dígitos exibido no server.
    Pin {
        /// O PIN, em texto (preserva zeros à esquerda).
        pin: Secret<String>,
    },
    /// Token de dispositivo já pareado (32 bytes).
    DeviceToken {
        /// O token.
        token: Secret<Bytes<32>>,
    },
}

/// `welcome` (S→C).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Welcome {
    /// Versão do protocolo da sessão.
    pub proto_ver: u8,
    /// Sessão UDP (1–2³¹−1).
    pub session_id: u32,
    /// Porta UDP de input.
    pub udp_port: u16,
    /// Chave ChaCha20-Poly1305 da sessão.
    pub udp_key: Secret<Bytes<32>>,
    /// Token do dispositivo; só no primeiro pareamento.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_token: Option<Secret<Bytes<32>>>,
    /// Identificador estável do server.
    pub server_id: Uuid,
    /// Nome do server (1–64).
    pub server_name: String,
    /// Configuração inicial.
    pub config: Config,
}

/// Configuração enviada ao client (no `welcome` e na mensagem `config`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Slot do controle (0–3).
    pub slot: u8,
    /// Taxa máxima de envio de input (10–500 Hz).
    pub send_rate_hz: u16,
    /// O server quer pacotes com motion (`0x02`).
    pub gyro_enabled: bool,
    /// O server vai mandar rumble.
    pub rumble_enabled: bool,
    /// Sugestão de vibração local ao tocar em botões.
    pub touch_haptics: bool,
}

/// Motivo de um `reject`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectReason {
    /// Mensagem malformada antes do `hello`.
    BadRequest,
    /// Credencial inválida ou expirada.
    BadAuth,
    /// Excesso de PINs errados.
    Locked,
    /// Dispositivo revogado.
    Revoked,
    /// Os 4 slots estão ocupados.
    NoSlot,
    /// `proto_ver` não suportado.
    Version,
    /// Motivo desconhecido nesta versão (tratar como erro genérico).
    #[serde(other)]
    Unknown,
}

/// `reject` (S→C).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reject {
    /// Motivo.
    pub reason: RejectReason,
    /// Segundos até poder tentar de novo (com `locked`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_s: Option<u32>,
    /// Versões aceitas pelo server (com `version`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supported_versions: Option<Vec<u8>>,
    /// Texto para depuração (≤ 256).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Motivo de um `bye`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ByeReason {
    /// O usuário encerrou.
    User,
    /// O app está fechando.
    Shutdown,
    /// O dispositivo foi revogado.
    Revoked,
    /// Desconectado pela UI do server.
    Kicked,
    /// Keepalive expirou.
    Timeout,
    /// O `seq` UDP esgotou (PROTOCOLO §6.3).
    SeqExhausted,
    /// Erro interno.
    Error,
    /// Motivo desconhecido nesta versão (tratar como erro genérico).
    #[serde(other)]
    Unknown,
}

/// `bye` (ambos).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bye {
    /// Motivo.
    pub reason: ByeReason,
    /// Texto para depuração (≤ 256).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// ---------------------------------------------------------------- Validação

fn check_len(field: &str, value: &str, min: usize, max: usize) -> Result<(), ControlError> {
    // JSON Schema conta code points, não bytes.
    let n = value.chars().count();
    if (min..=max).contains(&n) {
        Ok(())
    } else {
        Err(bad(format!("{field}: tamanho {n} fora de {min}..={max}")))
    }
}

fn check_range<T: Copy + PartialOrd + fmt::Display>(
    field: &str,
    v: T,
    min: T,
    max: T,
) -> Result<(), ControlError> {
    if v >= min && v <= max {
        Ok(())
    } else {
        Err(bad(format!("{field}: {v} fora de {min}..={max}")))
    }
}

fn check_message(value: Option<&String>) -> Result<(), ControlError> {
    value.map_or(Ok(()), |m| check_len("message", m, 0, 256))
}

impl Config {
    fn validate(self) -> Result<(), ControlError> {
        check_range("slot", self.slot, 0, 3)?;
        check_range("send_rate_hz", self.send_rate_hz, 10, 500)
    }
}

impl ControlMessage {
    /// Aplica as regras do schema que o serde não cobre (faixas e tamanhos).
    pub fn validate(&self) -> Result<(), ControlError> {
        match self {
            Self::Hello(h) => {
                check_range("proto_ver", h.proto_ver, 1, u8::MAX)?;
                check_len("name", &h.name, 1, 64)?;
                check_len("model", &h.model, 0, 64)?;
                check_len("os_ver", &h.os_ver, 0, 32)?;
                if let Some(v) = &h.app_ver {
                    check_len("app_ver", v, 0, 32)?;
                }
                if let Auth::Pin { pin } = &h.auth {
                    let pin = pin.expose();
                    if pin.len() != 6 || !pin.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(bad("pin: precisa de 6 dígitos"));
                    }
                }
                Ok(())
            }
            Self::Welcome(w) => {
                check_range("proto_ver", w.proto_ver, 1, u8::MAX)?;
                check_range("session_id", w.session_id, 1, 0x7FFF_FFFF)?;
                check_range("udp_port", w.udp_port, 1, u16::MAX)?;
                check_len("server_name", &w.server_name, 1, 64)?;
                w.config.validate()
            }
            Self::Reject(r) => {
                if let Some(s) = r.retry_after_s {
                    check_range("retry_after_s", s, 0, 3600)?;
                }
                if let Some(v) = &r.supported_versions
                    && (v.is_empty() || v.contains(&0))
                {
                    return Err(bad("supported_versions: vazio ou com versão 0"));
                }
                check_message(r.message.as_ref())
            }
            Self::Config(c) => c.validate(),
            Self::Bye(b) => check_message(b.message.as_ref()),
            Self::Ping | Self::Pong => Ok(()),
        }
    }
}

// ---------------------------------------------------------------- Framing

/// Resultado da leitura de um frame válido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Incoming {
    /// Mensagem conhecida e válida.
    Message(ControlMessage),
    /// `type` desconhecido nesta versão: deve ser ignorado (PROTOCOLO §5.2).
    Ignored,
}

/// Lê o prefixo de tamanho e devolve quantos bytes de corpo virão.
pub fn frame_len(prefix: [u8; FRAME_PREFIX_LEN]) -> Result<usize, ControlError> {
    let len = u32::from_le_bytes(prefix) as usize;
    if (1..=MAX_FRAME_LEN).contains(&len) {
        Ok(len)
    } else {
        Err(ControlError::BadFrame)
    }
}

/// Decodifica o corpo JSON de um frame.
pub fn decode_body(body: &[u8]) -> Result<Incoming, ControlError> {
    let value: serde_json::Value =
        serde_json::from_slice(body).map_err(|e| bad(format!("JSON inválido: {e}")))?;
    let Some(object) = value.as_object() else {
        return Err(bad("o JSON precisa ser um objeto"));
    };
    let Some(ty) = object.get("type").and_then(serde_json::Value::as_str) else {
        return Err(bad("type ausente ou não é texto"));
    };
    if !KNOWN_TYPES.contains(&ty) {
        return Ok(Incoming::Ignored);
    }
    let message: ControlMessage = serde_json::from_value(value).map_err(|e| bad(e.to_string()))?;
    message.validate()?;
    Ok(Incoming::Message(message))
}

/// Decodifica um frame completo (prefixo + corpo). Útil em testes e em buffers já lidos;
/// sockets devem usar [`frame_len`] e [`decode_body`].
pub fn decode_frame(frame: &[u8]) -> Result<Incoming, ControlError> {
    let Some(prefix) = frame.first_chunk::<FRAME_PREFIX_LEN>() else {
        return Err(ControlError::Incomplete);
    };
    let len = frame_len(*prefix)?;
    match frame.len() - FRAME_PREFIX_LEN {
        n if n < len => Err(ControlError::Incomplete),
        n if n > len => Err(ControlError::BadFrame),
        _ => decode_body(&frame[FRAME_PREFIX_LEN..]),
    }
}

/// Valida e serializa uma mensagem num frame pronto para enviar.
pub fn encode_frame(message: &ControlMessage) -> Result<Vec<u8>, ControlError> {
    message.validate()?;
    let body = serde_json::to_vec(message).map_err(|e| bad(e.to_string()))?;
    if body.len() > MAX_FRAME_LEN {
        return Err(ControlError::BadFrame);
    }
    let len = u32::try_from(body.len()).map_err(|_| ControlError::BadFrame)?;
    let mut frame = Vec::with_capacity(FRAME_PREFIX_LEN + body.len());
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&body);
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_e_volta() {
        let msg = ControlMessage::Config(Config {
            slot: 2,
            send_rate_hz: 250,
            gyro_enabled: true,
            rumble_enabled: false,
            touch_haptics: true,
        });
        let frame = encode_frame(&msg).unwrap();
        assert_eq!(decode_frame(&frame), Ok(Incoming::Message(msg)));
    }

    #[test]
    fn nao_codifica_mensagem_invalida() {
        let msg = ControlMessage::Config(Config {
            slot: 4,
            send_rate_hz: 250,
            gyro_enabled: false,
            rumble_enabled: false,
            touch_haptics: false,
        });
        assert_eq!(encode_frame(&msg).unwrap_err().code(), "bad_message");
    }

    #[test]
    fn frame_incompleto_e_com_sobra() {
        let frame = encode_frame(&ControlMessage::Ping).unwrap();
        assert_eq!(
            decode_frame(&frame[..frame.len() - 1]),
            Err(ControlError::Incomplete)
        );
        assert_eq!(decode_frame(&frame[..2]), Err(ControlError::Incomplete));
        let mut longer = frame.clone();
        longer.push(b' ');
        assert_eq!(decode_frame(&longer), Err(ControlError::BadFrame));
    }

    #[test]
    fn segredos_nao_vazam_no_debug() {
        let auth = Auth::Pin {
            pin: Secret::new("123456".into()),
        };
        assert!(!format!("{auth:?}").contains("123456"));
    }

    #[test]
    fn uuid() {
        assert!(Uuid::parse("3f2b8c1a-9d4e-4b7f-8a6c-1e2d3c4b5a69").is_ok());
        assert!(Uuid::parse("3F2B8C1A-9D4E-4B7F-8A6C-1E2D3C4B5A69").is_err());
        assert!(Uuid::parse("3f2b8c1a9d4e4b7f8a6c1e2d3c4b5a69").is_err());
    }
}
