//! Quem ocupa cada índice `XInput`: nossos controles virtuais ou controles físicos.

use vynpad_driver::MAX_SLOTS;

use crate::slots::SlotsSnapshot;

/// Ocupação de um índice `XInput`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XInputOccupant {
    /// Sem controle.
    Free,
    /// Um controle virtual do `VynPad`.
    Virtual,
    /// Um controle de outra origem (físico ou virtual de outro programa).
    Physical,
}

/// Classifica os 4 índices `XInput`.
///
/// `connected` vem da sonda do `XInput`; os índices dos nossos controles vêm do snapshot dos
/// slots. Conectado e que não é nosso é tratado como físico.
pub fn classify(
    connected: [bool; MAX_SLOTS as usize],
    snapshot: &SlotsSnapshot,
) -> [XInputOccupant; MAX_SLOTS as usize] {
    std::array::from_fn(|i| {
        let ours = snapshot
            .slots
            .iter()
            .any(|s| s.user_index.is_some_and(|u| usize::from(u) == i));
        match (connected[i], ours) {
            (false, _) => XInputOccupant::Free,
            (true, true) => XInputOccupant::Virtual,
            (true, false) => XInputOccupant::Physical,
        }
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use vynpad_driver::{MockDriver, SlotId, XReport};
    use vynpad_protocol::control::Uuid;

    use super::*;
    use crate::slots::SlotManager;

    #[test]
    fn separates_free_virtual_and_physical() {
        let driver = Arc::new(MockDriver::default());
        let manager = SlotManager::new(driver);
        let device = Uuid::parse("123e4567-e89b-42d3-a456-426614174000").unwrap();
        let slot = manager
            .assign(&device, 1, Some(SlotId::new(1).unwrap()))
            .unwrap();
        assert_eq!(slot.index(), 1);
        manager.submit(1, &XReport::default()).unwrap(); // o mock devolve índice = slot

        // XInput: 0 = físico, 1 = o nosso, 2 = físico, 3 = livre.
        let occupancy = classify([true, true, true, false], &manager.snapshot());
        assert_eq!(
            occupancy,
            [
                XInputOccupant::Physical,
                XInputOccupant::Virtual,
                XInputOccupant::Physical,
                XInputOccupant::Free
            ]
        );
    }

    #[test]
    fn our_index_without_a_connected_pad_is_free() {
        let manager = SlotManager::new(Arc::new(MockDriver::default()));
        let occupancy = classify([false; 4], &manager.snapshot());
        assert_eq!(occupancy, [XInputOccupant::Free; 4]);
    }
}
