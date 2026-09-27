//! Layout binário dos pacotes UDP (PROTOCOLO §6): cabeçalho e payloads em claro.
//!
//! Tudo aqui é little-endian, sem alocação e sem `unsafe`.

use crate::buttons::Buttons;
use crate::error::ProtoError;

/// `magic` do cabeçalho (bytes `50 47`, "PG").
pub const MAGIC: u16 = 0x4750;
/// Versão do protocolo (`ver` do cabeçalho e `proto_ver` do canal de controle).
pub const VERSION: u8 = 1;
/// Tamanho do cabeçalho em claro, autenticado como AAD.
pub const HEADER_LEN: usize = 12;
/// Tamanho da tag Poly1305.
pub const TAG_LEN: usize = 16;
/// Maior payload da v1 (`InputState` + `Motion`).
pub const MAX_PAYLOAD_LEN: usize = 36;
/// Maior pacote da v1; um buffer deste tamanho comporta qualquer pacote.
pub const MAX_PACKET_LEN: usize = HEADER_LEN + MAX_PAYLOAD_LEN + TAG_LEN;

const FLAG_MOTION: u8 = 0x01;

/// Direção de um pacote; entra no nonce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Direction {
    /// Client → server (`0`).
    ClientToServer = 0,
    /// Server → client (`1`).
    ServerToClient = 1,
}

/// Papel de quem está usando a sessão; define que direção se envia e qual se recebe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// O PC (recebe input, envia rumble e pong).
    Server,
    /// O celular ou o simulador (envia input, recebe rumble e pong).
    Client,
}

impl Role {
    /// Direção dos pacotes que este papel envia.
    pub const fn sends(self) -> Direction {
        match self {
            Self::Server => Direction::ServerToClient,
            Self::Client => Direction::ClientToServer,
        }
    }

    /// Direção dos pacotes que este papel recebe.
    pub const fn receives(self) -> Direction {
        match self {
            Self::Server => Direction::ClientToServer,
            Self::Client => Direction::ServerToClient,
        }
    }
}

/// Tipos de pacote da v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PacketType {
    /// `0x01` — [`InputState`].
    Input = 0x01,
    /// `0x02` — [`InputState`] + [`Motion`].
    InputMotion = 0x02,
    /// `0x10` — [`Ping`].
    Ping = 0x10,
    /// `0x11` — [`Pong`].
    Pong = 0x11,
    /// `0x20` — [`Rumble`].
    Rumble = 0x20,
}

impl PacketType {
    /// Converte o byte `type` do cabeçalho.
    pub const fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0x01 => Self::Input,
            0x02 => Self::InputMotion,
            0x10 => Self::Ping,
            0x11 => Self::Pong,
            0x20 => Self::Rumble,
            _ => return None,
        })
    }

    /// Direção fixa do tipo.
    pub const fn direction(self) -> Direction {
        match self {
            Self::Input | Self::InputMotion | Self::Ping => Direction::ClientToServer,
            Self::Pong | Self::Rumble => Direction::ServerToClient,
        }
    }

    /// Tamanho exato do payload em claro.
    pub const fn payload_len(self) -> usize {
        match self {
            Self::Input => 24,
            Self::InputMotion => 36,
            Self::Ping => 8,
            Self::Pong => 20,
            Self::Rumble => 4,
        }
    }

    /// Tamanho exato do pacote no fio (cabeçalho + payload + tag).
    pub const fn packet_len(self) -> usize {
        HEADER_LEN + self.payload_len() + TAG_LEN
    }
}

/// Cabeçalho de um pacote já validado (`magic` e `ver` são implícitos).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PacketHeader {
    /// Tipo do pacote.
    pub packet_type: PacketType,
    /// Sessão, vinda do `welcome`.
    pub session_id: u32,
    /// Contador da direção do pacote.
    pub seq: u32,
}

impl PacketHeader {
    /// Os 12 bytes do cabeçalho como vão no fio (e na AAD).
    pub fn to_bytes(&self) -> [u8; HEADER_LEN] {
        let mut h = [0u8; HEADER_LEN];
        h[0..2].copy_from_slice(&MAGIC.to_le_bytes());
        h[2] = VERSION;
        h[3] = self.packet_type as u8;
        h[4..8].copy_from_slice(&self.session_id.to_le_bytes());
        h[8..12].copy_from_slice(&self.seq.to_le_bytes());
        h
    }
}

/// Valida o cabeçalho de um datagrama recebido: passos 1 a 6 da PROTOCOLO §6.5.
///
/// Os passos seguintes (sessão, `seq`, tag e payload) ficam com quem conhece a sessão:
/// [`crate::open_packet`] ou [`crate::Session::receive`].
pub fn parse_header(packet: &[u8], role: Role) -> Result<PacketHeader, ProtoError> {
    let Some(h) = packet.first_chunk::<HEADER_LEN>() else {
        return Err(ProtoError::TooShort);
    };
    if u16::from_le_bytes([h[0], h[1]]) != MAGIC {
        return Err(ProtoError::BadMagic);
    }
    if h[2] != VERSION {
        return Err(ProtoError::BadVersion);
    }
    let packet_type = PacketType::from_u8(h[3]).ok_or(ProtoError::UnknownType)?;
    if packet_type.direction() != role.receives() {
        return Err(ProtoError::WrongDirection);
    }
    if packet.len() != packet_type.packet_len() {
        return Err(ProtoError::BadLength);
    }
    Ok(PacketHeader {
        packet_type,
        session_id: u32::from_le_bytes([h[4], h[5], h[6], h[7]]),
        seq: u32::from_le_bytes([h[8], h[9], h[10], h[11]]),
    })
}

/// Estado completo do controle enviado pelo client (`0x01`, e base do `0x02`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct InputState {
    /// Relógio monotônico do client, em µs.
    pub t_client_us: i64,
    /// Botões pressionados.
    pub buttons: Buttons,
    /// Gatilho esquerdo (0–255).
    pub lt: u8,
    /// Gatilho direito (0–255).
    pub rt: u8,
    /// Analógico esquerdo, X (positivo = direita).
    pub lx: i16,
    /// Analógico esquerdo, Y (positivo = cima).
    pub ly: i16,
    /// Analógico direito, X.
    pub rx: i16,
    /// Analógico direito, Y.
    pub ry: i16,
    /// `rumble_id` do último Rumble aplicado pelo client (`0` = nenhum).
    pub rumble_ack_id: u16,
    /// Bateria bruta: 0–100 (%), 255 = desconhecida. Use [`InputState::battery_percent`].
    pub battery: u8,
}

impl InputState {
    /// Valor de `battery` para "desconhecida".
    pub const BATTERY_UNKNOWN: u8 = 255;

    /// Bateria em %, ou `None` se desconhecida (101–255).
    pub const fn battery_percent(&self) -> Option<u8> {
        if self.battery <= 100 {
            Some(self.battery)
        } else {
            None
        }
    }

    fn write(&self, out: &mut [u8], motion: bool) {
        out[0..8].copy_from_slice(&self.t_client_us.to_le_bytes());
        out[8..10].copy_from_slice(&self.buttons.bits().to_le_bytes());
        out[10] = self.lt;
        out[11] = self.rt;
        out[12..14].copy_from_slice(&self.lx.to_le_bytes());
        out[14..16].copy_from_slice(&self.ly.to_le_bytes());
        out[16..18].copy_from_slice(&self.rx.to_le_bytes());
        out[18..20].copy_from_slice(&self.ry.to_le_bytes());
        out[20..22].copy_from_slice(&self.rumble_ack_id.to_le_bytes());
        out[22] = if motion { FLAG_MOTION } else { 0 };
        out[23] = self.battery;
    }

    /// Lê os 24 bytes e devolve o estado e o bit de motion.
    fn read(buf: &[u8]) -> (Self, bool) {
        let state = Self {
            t_client_us: i64_at(buf, 0),
            buttons: Buttons::from_wire(u16_at(buf, 8)),
            lt: buf[10],
            rt: buf[11],
            lx: i16_at(buf, 12),
            ly: i16_at(buf, 14),
            rx: i16_at(buf, 16),
            ry: i16_at(buf, 18),
            rumble_ack_id: u16_at(buf, 20),
            battery: buf[23],
        };
        (state, buf[22] & FLAG_MOTION != 0)
    }
}

/// Sensores de movimento no referencial do controle (PROTOCOLO §6.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Motion {
    /// Giroscópio X, rad/s × 1000.
    pub gx: i16,
    /// Giroscópio Y, rad/s × 1000.
    pub gy: i16,
    /// Giroscópio Z, rad/s × 1000.
    pub gz: i16,
    /// Acelerômetro X, m/s² × 100 (com gravidade).
    pub ax: i16,
    /// Acelerômetro Y, m/s² × 100.
    pub ay: i16,
    /// Acelerômetro Z, m/s² × 100.
    pub az: i16,
}

/// Ping do client (`0x10`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Ping {
    /// Relógio do client no envio, em µs.
    pub t_client_us: i64,
}

/// Pong do server (`0x11`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Pong {
    /// Eco do `t_client_us` do Ping.
    pub t_client_us: i64,
    /// Relógio do server na recepção do Ping, em µs.
    pub t_server_rx_us: i64,
    /// Tempo entre receber o Ping e enviar o Pong, em µs.
    pub server_proc_us: u32,
}

/// Rumble do server (`0x20`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rumble {
    /// Identificador do estado; muda a cada mudança, nunca é `0`.
    pub rumble_id: u16,
    /// Motor grande (baixa frequência), 0–255.
    pub large: u8,
    /// Motor pequeno (alta frequência), 0–255.
    pub small: u8,
}

impl Rumble {
    /// Próximo `rumble_id` depois de `id`: incrementa e pula o `0` ao dar a volta.
    pub const fn next_id(id: u16) -> u16 {
        match id.wrapping_add(1) {
            0 => 1,
            next => next,
        }
    }
}

/// Payload em claro de um pacote, já tipado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Payload {
    /// `0x01`.
    Input(InputState),
    /// `0x02`.
    InputMotion(InputState, Motion),
    /// `0x10`.
    Ping(Ping),
    /// `0x11`.
    Pong(Pong),
    /// `0x20`.
    Rumble(Rumble),
}

impl Payload {
    /// Tipo de pacote que carrega este payload.
    pub const fn packet_type(&self) -> PacketType {
        match self {
            Self::Input(_) => PacketType::Input,
            Self::InputMotion(..) => PacketType::InputMotion,
            Self::Ping(_) => PacketType::Ping,
            Self::Pong(_) => PacketType::Pong,
            Self::Rumble(_) => PacketType::Rumble,
        }
    }

    /// Serializa em `out`, que precisa ter exatamente `payload_len()` bytes.
    pub(crate) fn write(&self, out: &mut [u8]) {
        debug_assert_eq!(out.len(), self.packet_type().payload_len());
        match self {
            Self::Input(s) => s.write(out, false),
            Self::InputMotion(s, m) => {
                s.write(out, true);
                for (i, v) in [m.gx, m.gy, m.gz, m.ax, m.ay, m.az].into_iter().enumerate() {
                    out[24 + i * 2..26 + i * 2].copy_from_slice(&v.to_le_bytes());
                }
            }
            Self::Ping(p) => out.copy_from_slice(&p.t_client_us.to_le_bytes()),
            Self::Pong(p) => {
                out[0..8].copy_from_slice(&p.t_client_us.to_le_bytes());
                out[8..16].copy_from_slice(&p.t_server_rx_us.to_le_bytes());
                out[16..20].copy_from_slice(&p.server_proc_us.to_le_bytes());
            }
            Self::Rumble(r) => {
                out[0..2].copy_from_slice(&r.rumble_id.to_le_bytes());
                out[2] = r.large;
                out[3] = r.small;
            }
        }
    }

    /// Lê um payload em claro de `buf` (com exatamente `payload_len()` bytes). Passo 10 da §6.5.
    pub(crate) fn read(packet_type: PacketType, buf: &[u8]) -> Result<Self, ProtoError> {
        debug_assert_eq!(buf.len(), packet_type.payload_len());
        Ok(match packet_type {
            PacketType::Input | PacketType::InputMotion => {
                let (state, motion) = InputState::read(buf);
                if motion != (packet_type == PacketType::InputMotion) {
                    return Err(ProtoError::BadPayload);
                }
                if motion {
                    Self::InputMotion(
                        state,
                        Motion {
                            gx: i16_at(buf, 24),
                            gy: i16_at(buf, 26),
                            gz: i16_at(buf, 28),
                            ax: i16_at(buf, 30),
                            ay: i16_at(buf, 32),
                            az: i16_at(buf, 34),
                        },
                    )
                } else {
                    Self::Input(state)
                }
            }
            PacketType::Ping => Self::Ping(Ping {
                t_client_us: i64_at(buf, 0),
            }),
            PacketType::Pong => Self::Pong(Pong {
                t_client_us: i64_at(buf, 0),
                t_server_rx_us: i64_at(buf, 8),
                server_proc_us: u32_at(buf, 16),
            }),
            PacketType::Rumble => Self::Rumble(Rumble {
                rumble_id: u16_at(buf, 0),
                large: buf[2],
                small: buf[3],
            }),
        })
    }
}

/// Pacote decodificado e autenticado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Packet {
    /// Sessão do pacote.
    pub session_id: u32,
    /// `seq` do pacote.
    pub seq: u32,
    /// Conteúdo.
    pub payload: Payload,
}

fn u16_at(buf: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([buf[at], buf[at + 1]])
}

fn i16_at(buf: &[u8], at: usize) -> i16 {
    i16::from_le_bytes([buf[at], buf[at + 1]])
}

fn u32_at(buf: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]])
}

fn i64_at(buf: &[u8], at: usize) -> i64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&buf[at..at + 8]);
    i64::from_le_bytes(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tamanhos_da_spec() {
        assert_eq!(PacketType::Input.packet_len(), 52);
        assert_eq!(PacketType::InputMotion.packet_len(), 64);
        assert_eq!(PacketType::Ping.packet_len(), 36);
        assert_eq!(PacketType::Pong.packet_len(), 48);
        assert_eq!(PacketType::Rumble.packet_len(), 32);
        assert_eq!(MAX_PACKET_LEN, 64);
    }

    #[test]
    fn magic_e_pg_em_ascii() {
        let h = PacketHeader {
            packet_type: PacketType::Ping,
            session_id: 1,
            seq: 2,
        };
        assert_eq!(&h.to_bytes()[..2], b"PG");
    }

    #[test]
    fn rumble_id_pula_o_zero() {
        assert_eq!(Rumble::next_id(1), 2);
        assert_eq!(Rumble::next_id(0xFFFF), 1);
        assert_eq!(Rumble::next_id(0), 1);
    }

    #[test]
    fn bateria() {
        let mut s = InputState {
            battery: 100,
            ..InputState::default()
        };
        assert_eq!(s.battery_percent(), Some(100));
        s.battery = 101;
        assert_eq!(s.battery_percent(), None);
        s.battery = InputState::BATTERY_UNKNOWN;
        assert_eq!(s.battery_percent(), None);
    }
}
