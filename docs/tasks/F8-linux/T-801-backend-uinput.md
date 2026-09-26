# T-801 — Backend `uinput` (X360)

| Campo | Valor |
|---|---|
| Fase | F8 — Linux |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-201 |
| Status | ⬜ A fazer |

## Objetivo
No Linux, criar um controle que jogos, Steam e Proton reconheçam como um Xbox 360 real.

## Escopo
- Implementar `VirtualPadDriver` com o crate `evdev` (`VirtualDeviceBuilder` sobre `/dev/uinput`).
- Identidade: bus USB, vendor `0x045e`, product `0x028e`, versão `0x0110`, nome "Microsoft X-Box 360 pad" (para os mapeamentos do SDL/Steam funcionarem automaticamente).
- Eventos: `BTN_SOUTH/EAST/NORTH/WEST`, `BTN_TL/TR`, `BTN_SELECT/START/MODE`, `BTN_THUMBL/THUMBR`, `ABS_X/Y/RX/RY` (−32768..32767), `ABS_Z/RZ` (0..255), `ABS_HAT0X/HAT0Y` (D-pad), no mesmo layout do driver `xpad`.
- **Force feedback**: habilitar `FF_RUMBLE`, tratar `UI_FF_UPLOAD`/`UI_FF_ERASE` e os eventos `EV_FF` de play/stop → `RumbleEvent` (conferir o suporte no crate `evdev`; senão, ioctl direto isolado).
- `status()`: detectar a ausência de `/dev/uinput` ou a falta de permissão.

## Critérios de aceite
- [ ] `evtest` e `jstest-gtk` mostram todos os eixos e botões corretos.
- [ ] Steam reconhece como "Xbox 360 Controller"; funciona em um jogo nativo e em um jogo via Proton.
- [ ] Rumble chega ao `padgaming-sim` com `fftest`.

## Fora de escopo
- Permissões (T-802).
