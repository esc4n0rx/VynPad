//! Tipos que cruzam a ponte Tauri ↔ React. Os tipos de domínio ficam nos crates de núcleo, sem
//! saber do Tauri; aqui eles ganham a forma (serde + `specta::Type`) que o frontend recebe, e
//! `bindings.ts` é gerado a partir disto.

use std::time::Instant;

use padgaming_core::{SlotView, SlotsSnapshot};
use padgaming_driver::XReport;
use padgaming_mapping::Profile;
use padgaming_net::control::SessionHealth;
use padgaming_net::discovery::{InterfaceInfo, NearbyDevice};
use padgaming_server::{ApprovalPrompt, PairingOffer};
use padgaming_storage::{Device, DeviceDefaults, LogEntry, LogQuery, ProfileInput, ProfileSummary};
use serde::{Deserialize, Serialize};
use specta::Type;

// ------------------------------------------------------------------ slots

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SlotStateDto {
    Free,
    /// Reservado ao dispositivo, ainda sem controle plugado.
    Reserved,
    /// Controle plugado recebendo input.
    Active,
    /// A conexão caiu; o slot fica reservado até `expires_in_ms`.
    Suspended,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct SlotDto {
    pub slot: u8,
    pub state: SlotStateDto,
    /// Só em `suspended`: quanto falta para a reserva vencer.
    pub expires_in_ms: Option<u64>,
    pub device_id: Option<String>,
    pub session: Option<u32>,
    /// A UI pausou o slot.
    pub paused: bool,
    /// Índice `XInput` real do controle, quando conhecido.
    pub user_index: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct SlotsDto {
    pub slots: Vec<SlotDto>,
    pub free_count: u8,
}

impl From<&SlotsSnapshot> for SlotsDto {
    fn from(snapshot: &SlotsSnapshot) -> Self {
        let now = Instant::now();
        Self {
            slots: snapshot
                .slots
                .iter()
                .map(|s| {
                    let (state, expires_in_ms) = match s.state {
                        SlotView::Free => (SlotStateDto::Free, None),
                        SlotView::Reserved => (SlotStateDto::Reserved, None),
                        SlotView::Active => (SlotStateDto::Active, None),
                        SlotView::Suspended { expires_at } => (
                            SlotStateDto::Suspended,
                            Some(
                                u64::try_from(
                                    expires_at.saturating_duration_since(now).as_millis(),
                                )
                                .unwrap_or(u64::MAX),
                            ),
                        ),
                    };
                    SlotDto {
                        slot: s.slot.index(),
                        state,
                        expires_in_ms,
                        device_id: s.device.as_ref().map(|d| d.as_str().to_owned()),
                        session: s.session,
                        paused: s.paused,
                        user_index: s.user_index,
                    }
                })
                .collect(),
            free_count: u8::try_from(snapshot.free_count()).unwrap_or(4),
        }
    }
}

/// Estado do controle virtual, para o gamepad ao vivo (`input:live`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct InputReportDto {
    /// Máscara de botões `XInput`.
    pub buttons: u16,
    pub lt: u8,
    pub rt: u8,
    pub lx: i16,
    pub ly: i16,
    pub rx: i16,
    pub ry: i16,
}

impl From<XReport> for InputReportDto {
    fn from(r: XReport) -> Self {
        Self {
            buttons: r.buttons,
            lt: r.lt,
            rt: r.rt,
            lx: r.lx,
            ly: r.ly,
            rx: r.rx,
            ry: r.ry,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SessionHealthDto {
    Live,
    /// 250 ms sem pacotes: o controle foi solto.
    Neutral,
    /// 5 s sem pacotes: "Reconectando…".
    Stalled,
}

impl From<SessionHealth> for SessionHealthDto {
    fn from(h: SessionHealth) -> Self {
        match h {
            SessionHealth::Live => Self::Live,
            SessionHealth::Neutral => Self::Neutral,
            SessionHealth::Stalled => Self::Stalled,
        }
    }
}

// ------------------------------------------------------------------ perfis

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct ProfileSummaryDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub is_builtin: bool,
    pub schema_version: u32,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<ProfileSummary> for ProfileSummaryDto {
    fn from(p: ProfileSummary) -> Self {
        Self {
            id: p.id,
            name: p.name,
            description: p.description,
            icon: p.icon,
            is_builtin: p.is_builtin,
            schema_version: p.schema_version,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

/// Um perfil completo, com a configuração de mapeamento.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ProfileDto {
    pub summary: ProfileSummaryDto,
    pub profile: Profile,
}

impl From<padgaming_storage::StoredProfile> for ProfileDto {
    fn from(p: padgaming_storage::StoredProfile) -> Self {
        Self {
            summary: p.summary.into(),
            profile: p.profile,
        }
    }
}

/// O que o editor envia ao salvar ou criar um perfil.
#[derive(Debug, Clone, PartialEq, Deserialize, Type)]
pub struct ProfileInputDto {
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub profile: Profile,
}

impl From<ProfileInputDto> for ProfileInput {
    fn from(p: ProfileInputDto) -> Self {
        Self {
            name: p.name,
            description: p.description,
            icon: p.icon,
            profile: p.profile,
        }
    }
}

// ------------------------------------------------------------------ dispositivos

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct DeviceDto {
    pub id: String,
    pub name: String,
    pub model: Option<String>,
    pub os_version: Option<String>,
    pub paired_at: i64,
    pub last_seen_at: Option<i64>,
    pub last_ip: Option<String>,
    pub revoked: bool,
    pub preferred_slot: Option<u8>,
    pub default_profile_id: Option<String>,
    pub color: Option<String>,
    /// Slot em que o dispositivo está agora (com sessão ou reservado), se houver.
    pub current_slot: Option<u8>,
}

impl DeviceDto {
    pub fn new(device: Device, current_slot: Option<u8>) -> Self {
        Self {
            id: device.id,
            name: device.name,
            model: device.model,
            os_version: device.os_version,
            paired_at: device.paired_at,
            last_seen_at: device.last_seen_at,
            last_ip: device.last_ip,
            revoked: device.revoked,
            preferred_slot: device.preferred_slot,
            default_profile_id: device.default_profile_id,
            color: device.color,
            current_slot,
        }
    }
}

/// Preferências de um dispositivo; `null` limpa o campo.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Type)]
pub struct DeviceDefaultsDto {
    pub preferred_slot: Option<u8>,
    pub default_profile_id: Option<String>,
    pub color: Option<String>,
}

impl From<DeviceDefaultsDto> for DeviceDefaults {
    fn from(d: DeviceDefaultsDto) -> Self {
        Self {
            preferred_slot: d.preferred_slot,
            default_profile_id: d.default_profile_id,
            color: d.color,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct LogEntryDto {
    pub id: i64,
    pub device_id: Option<String>,
    /// `connected`, `disconnected`, `rejected`, `paired` ou `revoked`.
    pub event: String,
    pub slot: Option<u8>,
    pub detail: Option<String>,
    pub at: i64,
}

impl From<LogEntry> for LogEntryDto {
    fn from(e: LogEntry) -> Self {
        Self {
            id: e.id,
            device_id: e.device_id,
            event: e.event,
            slot: e.slot,
            detail: e.detail,
            at: e.at,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Type)]
pub struct LogQueryDto {
    pub device_id: Option<String>,
    /// Só eventos anteriores a este `id` (próxima página).
    pub before_id: Option<i64>,
    /// 1–500; 0 ou ausente vale 100.
    pub limit: Option<u32>,
}

impl From<LogQueryDto> for LogQuery {
    fn from(q: LogQueryDto) -> Self {
        Self {
            device_id: q.device_id,
            before_id: q.before_id,
            limit: q.limit.unwrap_or(0),
        }
    }
}

// ------------------------------------------------------------------ pareamento e rede

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct PairingOfferDto {
    /// URI do QR code.
    pub uri: String,
    /// O QR code em SVG.
    pub qr_svg: String,
    /// PIN de 6 dígitos.
    pub pin: String,
    pub token_ttl_s: u32,
    pub pin_ttl_s: u32,
    /// 8 primeiros bytes do fingerprint em hex, para conferir com o celular.
    pub fingerprint_short: String,
    pub hosts: Vec<String>,
    pub port: u16,
}

impl From<PairingOffer> for PairingOfferDto {
    fn from(o: PairingOffer) -> Self {
        Self {
            uri: o.uri,
            qr_svg: o.qr_svg,
            pin: o.pin,
            token_ttl_s: u32::try_from(o.token_ttl.as_secs()).unwrap_or(u32::MAX),
            pin_ttl_s: u32::try_from(o.pin_ttl.as_secs()).unwrap_or(u32::MAX),
            fingerprint_short: o.fingerprint_short,
            hosts: o.hosts.iter().map(ToString::to_string).collect(),
            port: o.port,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct NearbyDto {
    pub id: String,
    pub name: String,
    pub model: String,
    pub ip: String,
    pub port: u16,
    pub seen_ago_ms: u64,
    /// O aparelho já está pareado neste server.
    pub paired: bool,
}

impl From<NearbyDevice> for NearbyDto {
    fn from(d: NearbyDevice) -> Self {
        Self {
            id: d.id.as_str().to_owned(),
            name: d.name,
            model: d.model,
            ip: d.ip.to_string(),
            port: d.port,
            seen_ago_ms: u64::try_from(d.seen_ago.as_millis()).unwrap_or(u64::MAX),
            paired: d.paired,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct AddressDto {
    pub interface: String,
    pub ip: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct InterfaceDto {
    pub name: String,
    pub ip: String,
    /// O modo automático usa esta interface.
    pub auto: bool,
    /// Parece virtual (Hyper-V, WSL, Docker, VPN...).
    pub virtual_like: bool,
}

impl From<InterfaceInfo> for InterfaceDto {
    fn from(i: InterfaceInfo) -> Self {
        Self {
            name: i.name,
            ip: i.ip.to_string(),
            auto: i.auto,
            virtual_like: i.virtual_like,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct ServerInfoDto {
    pub name: String,
    pub fingerprint_short: String,
    pub control_port: u16,
    pub input_port: u16,
    pub discovery_port: u16,
    /// Portas configuradas que só valem depois de reiniciar o app.
    pub ports_pending_restart: Option<padgaming_storage::Ports>,
    pub addresses: Vec<AddressDto>,
    pub free_slots: u8,
}

// ------------------------------------------------------------------ confirmação

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ApprovalPromptDto {
    pub request_id: u64,
    pub device_id: String,
    pub name: String,
    pub model: String,
    pub peer: String,
}

impl From<ApprovalPrompt> for ApprovalPromptDto {
    fn from(p: ApprovalPrompt) -> Self {
        Self {
            request_id: p.request_id,
            device_id: p.device_id,
            name: p.name,
            model: p.model,
            peer: p.peer.to_string(),
        }
    }
}
