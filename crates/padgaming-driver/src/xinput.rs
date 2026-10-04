//! Ocupação dos 4 slots `XInput` do Windows.
//!
//! O Windows só expõe 4 controles `XInput` ao mesmo tempo, somando físicos e virtuais.
//! Esta sonda diz quais índices têm um controle conectado; quem sabe quais são os nossos
//! (o `SlotManager`) separa os físicos.

use rusty_xinput::XInputHandle;

use crate::MAX_SLOTS;

/// Consulta `XInputGetState` para os índices 0..3.
pub struct XInputProbe {
    handle: XInputHandle,
}

impl XInputProbe {
    /// Carrega a DLL do `XInput`. `None` se o sistema não a tiver.
    pub fn load() -> Option<Self> {
        XInputHandle::load_default()
            .ok()
            .map(|handle| Self { handle })
    }

    /// Quais índices `XInput` têm um controle conectado (físico ou virtual).
    pub fn connected(&self) -> [bool; MAX_SLOTS as usize] {
        std::array::from_fn(|i| self.handle.get_state(u32::try_from(i).unwrap_or(0)).is_ok())
    }

    /// O botão A do controle do índice `XInput` está apertado (`None` se não há controle ali).
    /// Serve para medir a latência até o `XInput` (T-702).
    pub fn a_pressed(&self, index: u8) -> Option<bool> {
        self.handle
            .get_state(u32::from(index))
            .ok()
            .map(|state| state.south_button())
    }
}

impl std::fmt::Debug for XInputProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("XInputProbe")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_answers_for_all_four_indices() {
        // Em máquinas sem XInput a sonda simplesmente não existe.
        if let Some(probe) = XInputProbe::load() {
            assert_eq!(probe.connected().len(), 4);
        }
    }
}
