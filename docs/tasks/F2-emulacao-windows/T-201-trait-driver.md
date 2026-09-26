# T-201 — Trait `VirtualPadDriver` + driver mock

| Campo | Valor |
|---|---|
| Fase | F2 — Emulação Windows |
| Prioridade | P0 |
| Estimativa | P |
| Depende de | T-001 |
| Status | ⬜ A fazer |

## Objetivo
Isolar o backend de emulação (ViGEm no Windows, uinput no Linux) atrás de uma interface comum, testável sem driver instalado.

## Escopo
```rust
pub struct XReport { pub buttons: u16, pub lt: u8, pub rt: u8, pub lx: i16, pub ly: i16, pub rx: i16, pub ry: i16 }
pub struct RumbleEvent { pub slot: SlotId, pub large: u8, pub small: u8 }

pub trait VirtualPadDriver: Send + Sync {
    fn status(&self) -> DriverStatus;               // Ready | NotInstalled | Outdated{..} | Error(..)
    fn plug(&self, slot: SlotId) -> Result<PadHandle, DriverError>;
    fn unplug(&self, handle: PadHandle) -> Result<(), DriverError>;
    fn update(&self, handle: &PadHandle, report: &XReport) -> Result<(), DriverError>;
    fn user_index(&self, handle: &PadHandle) -> Option<u8>; // índice XInput real (0..3)
    fn rumble_events(&self) -> crossbeam_channel::Receiver<RumbleEvent>;
}
```
- `MockDriver` (feature `mock`) grava os reports para os testes e permite injetar rumble.
- Seleção do backend por `cfg(target_os)` numa factory `default_driver()`.

## Critérios de aceite
- [ ] O `padgaming-core` compila e testa usando só o `MockDriver`.
- [ ] `update()` não aloca (verificado no teste com um allocator contador).

## Fora de escopo
- Implementações reais (T-202, T-801).
