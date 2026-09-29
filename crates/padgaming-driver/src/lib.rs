//! Trait `VirtualPadDriver` e backends de emulação do X360 (`ViGEmBus` no Windows, `uinput` no Linux).
//!
//! O resto do server só conhece a trait; o backend concreto sai de [`default_driver`].
//! Com a feature `mock`, [`MockDriver`] permite testar sem driver instalado.

use crossbeam_channel::Receiver;

#[cfg(any(test, feature = "mock"))]
mod mock;
#[cfg(any(test, feature = "mock"))]
pub use mock::MockDriver;

/// Quantidade de controles simultâneos.
pub const MAX_SLOTS: u8 = 4;

/// Máscaras de `XReport::buttons` (mesmos bits de `XINPUT_GAMEPAD`).
pub mod xbuttons {
    /// D-pad para cima.
    pub const DPAD_UP: u16 = 0x0001;
    /// D-pad para baixo.
    pub const DPAD_DOWN: u16 = 0x0002;
    /// D-pad para a esquerda.
    pub const DPAD_LEFT: u16 = 0x0004;
    /// D-pad para a direita.
    pub const DPAD_RIGHT: u16 = 0x0008;
    /// Start.
    pub const START: u16 = 0x0010;
    /// Back.
    pub const BACK: u16 = 0x0020;
    /// Clique do analógico esquerdo.
    pub const LEFT_THUMB: u16 = 0x0040;
    /// Clique do analógico direito.
    pub const RIGHT_THUMB: u16 = 0x0080;
    /// LB.
    pub const LEFT_SHOULDER: u16 = 0x0100;
    /// RB.
    pub const RIGHT_SHOULDER: u16 = 0x0200;
    /// Botão Guide (Xbox).
    pub const GUIDE: u16 = 0x0400;
    /// A.
    pub const A: u16 = 0x1000;
    /// B.
    pub const B: u16 = 0x2000;
    /// X.
    pub const X: u16 = 0x4000;
    /// Y.
    pub const Y: u16 = 0x8000;
}

/// Posição do controle virtual (0–3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SlotId(u8);

impl SlotId {
    /// `Some` se `index` está em `0..MAX_SLOTS`.
    pub const fn new(index: u8) -> Option<Self> {
        if index < MAX_SLOTS {
            Some(Self(index))
        } else {
            None
        }
    }

    /// O índice do slot (0–3).
    pub const fn index(self) -> u8 {
        self.0
    }

    /// Os quatro slots, em ordem.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..MAX_SLOTS).map(Self)
    }
}

/// Estado do controle X360 (o `XINPUT_GAMEPAD`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct XReport {
    /// Bits de [`xbuttons`].
    pub buttons: u16,
    /// Gatilho esquerdo (0–255).
    pub lt: u8,
    /// Gatilho direito (0–255).
    pub rt: u8,
    /// Analógico esquerdo, eixo X.
    pub lx: i16,
    /// Analógico esquerdo, eixo Y.
    pub ly: i16,
    /// Analógico direito, eixo X.
    pub rx: i16,
    /// Analógico direito, eixo Y.
    pub ry: i16,
}

/// Vibração pedida pelo jogo para um slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RumbleEvent {
    /// Slot que recebeu a vibração.
    pub slot: SlotId,
    /// Motor grave (0–255).
    pub large: u8,
    /// Motor agudo (0–255).
    pub small: u8,
}

/// Disponibilidade do backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverStatus {
    /// Pronto para criar controles.
    Ready,
    /// O driver (`ViGEmBus`) não está instalado.
    NotInstalled,
    /// Instalado, mas em versão anterior à exigida.
    Outdated {
        /// Versão encontrada.
        found: String,
        /// Versão mínima exigida.
        required: String,
    },
    /// Qualquer outra falha.
    Error(String),
}

/// Erro de uma operação do driver.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DriverError {
    /// O backend não está pronto (ver [`VirtualPadDriver::status`]).
    #[error("driver indisponível: {0:?}")]
    NotReady(DriverStatus),
    /// O slot já tem um controle ligado.
    #[error("slot {} já está em uso", .0.index())]
    SlotOccupied(SlotId),
    /// O handle não corresponde a um controle ligado (já removido, por exemplo).
    #[error("handle inválido")]
    InvalidHandle,
    /// Falha do backend, com o texto dele.
    #[error("falha do backend: {0}")]
    Backend(String),
}

/// Controle virtual ligado. Não é `Clone`: [`VirtualPadDriver::unplug`] o consome.
#[derive(Debug, PartialEq, Eq)]
pub struct PadHandle {
    slot: SlotId,
    /// Distingue ligações sucessivas do mesmo slot.
    token: u64,
}

impl PadHandle {
    /// Para os backends: cria um handle.
    pub const fn new(slot: SlotId, token: u64) -> Self {
        Self { slot, token }
    }

    /// Slot do controle.
    pub const fn slot(&self) -> SlotId {
        self.slot
    }

    /// Identificador da ligação, definido pelo backend.
    pub const fn token(&self) -> u64 {
        self.token
    }
}

/// Backend de emulação do controle Xbox 360.
pub trait VirtualPadDriver: Send + Sync {
    /// Disponibilidade atual do backend.
    fn status(&self) -> DriverStatus;

    /// Liga um controle virtual no slot.
    fn plug(&self, slot: SlotId) -> Result<PadHandle, DriverError>;

    /// Desliga o controle.
    fn unplug(&self, handle: PadHandle) -> Result<(), DriverError>;

    /// Envia o estado ao controle. Roda no hot path: **não pode alocar**.
    fn update(&self, handle: &PadHandle, report: &XReport) -> Result<(), DriverError>;

    /// Índice `XInput` real (0–3) que o Windows atribuiu ao controle, se já for conhecido.
    fn user_index(&self, handle: &PadHandle) -> Option<u8>;

    /// Canal de vibração vinda dos jogos (todos os slots).
    fn rumble_events(&self) -> Receiver<RumbleEvent>;
}

/// Backend da plataforma atual.
///
/// Ainda sem backends reais (T-202 no Windows, T-801 no Linux): devolve um driver que
/// se declara indisponível, para o resto do app já tratar o caso.
pub fn default_driver() -> Box<dyn VirtualPadDriver> {
    Box::new(Unavailable)
}

struct Unavailable;

impl Unavailable {
    fn status() -> DriverStatus {
        DriverStatus::Error("nenhum backend de emulação para esta plataforma".to_owned())
    }
}

impl VirtualPadDriver for Unavailable {
    fn status(&self) -> DriverStatus {
        Self::status()
    }

    fn plug(&self, _: SlotId) -> Result<PadHandle, DriverError> {
        Err(DriverError::NotReady(Self::status()))
    }

    fn unplug(&self, _: PadHandle) -> Result<(), DriverError> {
        Err(DriverError::InvalidHandle)
    }

    fn update(&self, _: &PadHandle, _: &XReport) -> Result<(), DriverError> {
        Err(DriverError::InvalidHandle)
    }

    fn user_index(&self, _: &PadHandle) -> Option<u8> {
        None
    }

    fn rumble_events(&self) -> Receiver<RumbleEvent> {
        crossbeam_channel::never()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_id_accepts_only_0_to_3() {
        assert!(SlotId::new(3).is_some());
        assert!(SlotId::new(4).is_none());
        assert_eq!(
            SlotId::all().map(SlotId::index).collect::<Vec<_>>(),
            [0, 1, 2, 3]
        );
    }

    #[test]
    fn default_driver_reports_unavailable_until_a_backend_exists() {
        let driver = default_driver();
        assert!(matches!(driver.status(), DriverStatus::Error(_)));
        let slot = SlotId::new(0).unwrap();
        assert!(matches!(driver.plug(slot), Err(DriverError::NotReady(_))));
    }
}
