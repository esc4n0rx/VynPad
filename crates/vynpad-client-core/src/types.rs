//! Tipos de dados trocados com a UI.

use vynpad_protocol::{Buttons, InputState, Motion, Payload};

use crate::CoreError;

/// Estado completo do controle, como a UI o lê dos toques.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, uniffi::Record)]
pub struct GamepadInput {
    /// Relógio monotônico do client, em µs.
    pub t_client_us: i64,
    /// Botões no formato `XInput` (bits reservados são descartados).
    pub buttons: u16,
    /// Gatilho esquerdo, 0–255.
    pub lt: u8,
    /// Gatilho direito, 0–255.
    pub rt: u8,
    /// Analógico esquerdo, X (positivo = direita).
    pub lx: i16,
    /// Analógico esquerdo, Y (positivo = cima).
    pub ly: i16,
    /// Analógico direito, X.
    pub rx: i16,
    /// Analógico direito, Y.
    pub ry: i16,
    /// `rumble_id` do último rumble aplicado (`0` = nenhum).
    pub rumble_ack_id: u16,
    /// Bateria em %, ou 255 se desconhecida.
    pub battery: u8,
}

impl From<GamepadInput> for InputState {
    fn from(i: GamepadInput) -> Self {
        Self {
            t_client_us: i.t_client_us,
            buttons: Buttons::from_wire(i.buttons),
            lt: i.lt,
            rt: i.rt,
            lx: i.lx,
            ly: i.ly,
            rx: i.rx,
            ry: i.ry,
            rumble_ack_id: i.rumble_ack_id,
            battery: i.battery,
        }
    }
}

/// Amostra de giroscópio e acelerômetro (PROTOCOLO §6.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, uniffi::Record)]
pub struct MotionSample {
    /// Giroscópio X, rad/s × 1000.
    pub gx: i16,
    /// Giroscópio Y, rad/s × 1000.
    pub gy: i16,
    /// Giroscópio Z, rad/s × 1000.
    pub gz: i16,
    /// Acelerômetro X, m/s² × 100.
    pub ax: i16,
    /// Acelerômetro Y, m/s² × 100.
    pub ay: i16,
    /// Acelerômetro Z, m/s² × 100.
    pub az: i16,
}

impl From<MotionSample> for Motion {
    fn from(m: MotionSample) -> Self {
        Self {
            gx: m.gx,
            gy: m.gy,
            gz: m.gz,
            ax: m.ax,
            ay: m.ay,
            az: m.az,
        }
    }
}

/// O que o client pode receber do server por UDP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ReceivedPacket {
    /// Resposta a um ping, para medir o RTT.
    Pong {
        /// Eco do `t_client_us` do ping.
        t_client_us: i64,
        /// Relógio do server na recepção do ping, em µs.
        t_server_rx_us: i64,
        /// Tempo entre receber o ping e responder, em µs.
        server_proc_us: u32,
    },
    /// Comando de vibração vindo do jogo.
    Rumble {
        /// Identificador do estado (nunca `0`); devolver em `rumble_ack_id`.
        rumble_id: u16,
        /// Motor grande (baixa frequência), 0–255.
        large: u8,
        /// Motor pequeno (alta frequência), 0–255.
        small: u8,
    },
}

impl TryFrom<Payload> for ReceivedPacket {
    type Error = CoreError;

    fn try_from(payload: Payload) -> Result<Self, CoreError> {
        match payload {
            Payload::Pong(p) => Ok(Self::Pong {
                t_client_us: p.t_client_us,
                t_server_rx_us: p.t_server_rx_us,
                server_proc_us: p.server_proc_us,
            }),
            Payload::Rumble(r) => Ok(Self::Rumble {
                rumble_id: r.rumble_id,
                large: r.large,
                small: r.small,
            }),
            // O client só recebe Pong e Rumble; o resto é barrado antes pela direção.
            Payload::Input(_) | Payload::InputMotion(..) | Payload::Ping(_) => {
                Err(vynpad_protocol::ProtoError::WrongDirection.into())
            }
        }
    }
}
