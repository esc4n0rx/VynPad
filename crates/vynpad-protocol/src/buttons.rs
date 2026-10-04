//! Bitmask de botões no formato `XInput` (PROTOCOLO §7).

bitflags::bitflags! {
    /// Botões pressionados. Os bits são os mesmos do `XINPUT_GAMEPAD.wButtons`,
    /// então o valor vai direto para o driver virtual.
    ///
    /// O bit `0x0800` é reservado: [`Buttons::from_wire`] o descarta.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Buttons: u16 {
        /// D-pad para cima.
        const DPAD_UP = 0x0001;
        /// D-pad para baixo.
        const DPAD_DOWN = 0x0002;
        /// D-pad para a esquerda.
        const DPAD_LEFT = 0x0004;
        /// D-pad para a direita.
        const DPAD_RIGHT = 0x0008;
        /// Start.
        const START = 0x0010;
        /// Back.
        const BACK = 0x0020;
        /// Clique do analógico esquerdo (L3).
        const LEFT_THUMB = 0x0040;
        /// Clique do analógico direito (R3).
        const RIGHT_THUMB = 0x0080;
        /// Bumper esquerdo (LB).
        const LEFT_SHOULDER = 0x0100;
        /// Bumper direito (RB).
        const RIGHT_SHOULDER = 0x0200;
        /// Guide (`XUSB_GAMEPAD_GUIDE` no ViGEm).
        const GUIDE = 0x0400;
        /// A.
        const A = 0x1000;
        /// B.
        const B = 0x2000;
        /// X.
        const X = 0x4000;
        /// Y.
        const Y = 0x8000;
    }
}

impl Buttons {
    /// Converte o valor lido do fio, mascarando os bits reservados.
    pub const fn from_wire(bits: u16) -> Self {
        Self::from_bits_truncate(bits)
    }

    /// Nome do botão usado nos perfis de mapeamento e nos test vectors (`a`, `dpad_up`…).
    ///
    /// Devolve `None` se `self` não for exatamente um botão.
    pub fn name(self) -> Option<&'static str> {
        Some(match self {
            Self::DPAD_UP => "dpad_up",
            Self::DPAD_DOWN => "dpad_down",
            Self::DPAD_LEFT => "dpad_left",
            Self::DPAD_RIGHT => "dpad_right",
            Self::START => "start",
            Self::BACK => "back",
            Self::LEFT_THUMB => "left_thumb",
            Self::RIGHT_THUMB => "right_thumb",
            Self::LEFT_SHOULDER => "left_shoulder",
            Self::RIGHT_SHOULDER => "right_shoulder",
            Self::GUIDE => "guide",
            Self::A => "a",
            Self::B => "b",
            Self::X => "x",
            Self::Y => "y",
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mascara_o_bit_reservado() {
        assert_eq!(Buttons::from_wire(0xFFFF).bits(), 0xF7FF);
        assert_eq!(Buttons::all().bits(), 0xF7FF);
    }

    #[test]
    fn todo_botao_tem_nome() {
        for b in Buttons::all().iter() {
            assert!(b.name().is_some(), "{b:?}");
        }
        assert_eq!((Buttons::A | Buttons::B).name(), None);
    }
}
