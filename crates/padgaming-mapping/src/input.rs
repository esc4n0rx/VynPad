//! Entrada bruta do client e os nomes dos botões usados nos perfis.

use padgaming_driver::xbuttons;
use padgaming_protocol::{InputState, Motion};
use serde::{Deserialize, Serialize};

/// Um botão do controle X360. Os nomes JSON são os mesmos do protocolo (`a`, `dpad_up`...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Button {
    /// D-pad para cima.
    DpadUp,
    /// D-pad para baixo.
    DpadDown,
    /// D-pad para a esquerda.
    DpadLeft,
    /// D-pad para a direita.
    DpadRight,
    /// Start.
    Start,
    /// Back.
    Back,
    /// Clique do analógico esquerdo.
    LeftThumb,
    /// Clique do analógico direito.
    RightThumb,
    /// LB.
    LeftShoulder,
    /// RB.
    RightShoulder,
    /// Guide.
    Guide,
    /// A.
    A,
    /// B.
    B,
    /// X.
    X,
    /// Y.
    Y,
}

/// Quantidade de botões.
pub const BUTTON_COUNT: usize = 15;

impl Button {
    /// Todos os botões, na ordem usada pelas tabelas internas.
    pub const ALL: [Self; BUTTON_COUNT] = [
        Self::DpadUp,
        Self::DpadDown,
        Self::DpadLeft,
        Self::DpadRight,
        Self::Start,
        Self::Back,
        Self::LeftThumb,
        Self::RightThumb,
        Self::LeftShoulder,
        Self::RightShoulder,
        Self::Guide,
        Self::A,
        Self::B,
        Self::X,
        Self::Y,
    ];

    /// Bit do botão em `XReport::buttons`.
    pub const fn mask(self) -> u16 {
        match self {
            Self::DpadUp => xbuttons::DPAD_UP,
            Self::DpadDown => xbuttons::DPAD_DOWN,
            Self::DpadLeft => xbuttons::DPAD_LEFT,
            Self::DpadRight => xbuttons::DPAD_RIGHT,
            Self::Start => xbuttons::START,
            Self::Back => xbuttons::BACK,
            Self::LeftThumb => xbuttons::LEFT_THUMB,
            Self::RightThumb => xbuttons::RIGHT_THUMB,
            Self::LeftShoulder => xbuttons::LEFT_SHOULDER,
            Self::RightShoulder => xbuttons::RIGHT_SHOULDER,
            Self::Guide => xbuttons::GUIDE,
            Self::A => xbuttons::A,
            Self::B => xbuttons::B,
            Self::X => xbuttons::X,
            Self::Y => xbuttons::Y,
        }
    }

    /// Posição em [`Button::ALL`].
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Nome usado nos perfis JSON.
    pub const fn name(self) -> &'static str {
        match self {
            Self::DpadUp => "dpad_up",
            Self::DpadDown => "dpad_down",
            Self::DpadLeft => "dpad_left",
            Self::DpadRight => "dpad_right",
            Self::Start => "start",
            Self::Back => "back",
            Self::LeftThumb => "left_thumb",
            Self::RightThumb => "right_thumb",
            Self::LeftShoulder => "left_shoulder",
            Self::RightShoulder => "right_shoulder",
            Self::Guide => "guide",
            Self::A => "a",
            Self::B => "b",
            Self::X => "x",
            Self::Y => "y",
        }
    }

    /// Inverso de [`Button::name`].
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|b| b.name() == name)
    }

    /// União dos bits de todos os botões conhecidos.
    pub const ALL_MASK: u16 = xbuttons::DPAD_UP
        | xbuttons::DPAD_DOWN
        | xbuttons::DPAD_LEFT
        | xbuttons::DPAD_RIGHT
        | xbuttons::START
        | xbuttons::BACK
        | xbuttons::LEFT_THUMB
        | xbuttons::RIGHT_THUMB
        | xbuttons::LEFT_SHOULDER
        | xbuttons::RIGHT_SHOULDER
        | xbuttons::GUIDE
        | xbuttons::A
        | xbuttons::B
        | xbuttons::X
        | xbuttons::Y;
}

/// Velocidade angular no referencial do controle (PROTOCOLO §6.4), em rad/s.
///
/// X para a direita, Y para cima, Z saindo da tela; rotação positiva = anti-horária.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Gyro {
    /// Em torno de X (inclinar o topo do aparelho para o jogador é positivo).
    pub x: f32,
    /// Em torno de Y (virar para a esquerda é positivo).
    pub y: f32,
    /// Em torno de Z (inclinar para a esquerda é positivo).
    pub z: f32,
}

/// Estado bruto de um controle, antes do perfil.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RawInput {
    /// Bits de `xbuttons`.
    pub buttons: u16,
    /// Gatilho esquerdo.
    pub lt: u8,
    /// Gatilho direito.
    pub rt: u8,
    /// Analógico esquerdo X.
    pub lx: i16,
    /// Analógico esquerdo Y (positivo = cima).
    pub ly: i16,
    /// Analógico direito X.
    pub rx: i16,
    /// Analógico direito Y.
    pub ry: i16,
    /// Giroscópio, se o pacote trouxe motion.
    pub gyro: Option<Gyro>,
}

impl RawInput {
    /// Monta a entrada a partir dos pacotes UDP (`0x01` ou `0x02`).
    pub fn from_wire(input: &InputState, motion: Option<&Motion>) -> Self {
        Self {
            buttons: input.buttons.bits(),
            lt: input.lt,
            rt: input.rt,
            lx: input.lx,
            ly: input.ly,
            rx: input.rx,
            ry: input.ry,
            gyro: motion.map(|m| Gyro {
                x: f32::from(m.gx) / 1000.0,
                y: f32::from(m.gy) / 1000.0,
                z: f32::from(m.gz) / 1000.0,
            }),
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // comparações exatas são intencionais nestes testes
mod tests {
    use padgaming_protocol::Buttons;

    use super::*;

    #[test]
    fn names_masks_and_indices_are_consistent() {
        for (i, button) in Button::ALL.into_iter().enumerate() {
            assert_eq!(button.index(), i);
            assert_eq!(Button::from_name(button.name()), Some(button));
            assert_eq!(button.mask().count_ones(), 1);
            // Os nomes batem com os do protocolo.
            let wire = Buttons::from_bits_truncate(button.mask());
            assert_eq!(wire.name(), Some(button.name()));
        }
        assert_eq!(Button::from_name("nope"), None);
        assert_eq!(Button::ALL_MASK, Buttons::all().bits());
    }

    #[test]
    fn wire_conversion_scales_gyro_to_rad_per_second() {
        let input = InputState {
            buttons: Buttons::A | Buttons::START,
            lt: 1,
            rt: 2,
            lx: 3,
            ly: -4,
            rx: 5,
            ry: -6,
            ..InputState::default()
        };
        let motion = Motion {
            gx: 1500,
            gy: -2000,
            gz: 250,
            ..Motion::default()
        };
        let raw = RawInput::from_wire(&input, Some(&motion));
        assert_eq!(raw.buttons, xbuttons::A | xbuttons::START);
        assert_eq!(
            (raw.lt, raw.rt, raw.lx, raw.ly, raw.rx, raw.ry),
            (1, 2, 3, -4, 5, -6)
        );
        assert_eq!(
            raw.gyro,
            Some(Gyro {
                x: 1.5,
                y: -2.0,
                z: 0.25
            })
        );
        assert_eq!(RawInput::from_wire(&input, None).gyro, None);
    }
}
