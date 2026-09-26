# T-202 — Backend ViGEmBus

| Campo | Valor |
|---|---|
| Fase | F2 — Emulação Windows |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-201 |
| Status | ⬜ A fazer |

## Objetivo
Criar controles Xbox 360 virtuais reais, reconhecidos pelo Windows e por qualquer jogo XInput.

## Escopo
- Implementar `VirtualPadDriver` com o crate `vigem-client` (`Client::connect`, `Xbox360Wired`, `plugin`, `wait_ready`, `update`).
- Um `Client` compartilhado; um target por slot.
- Mapear `XReport` → `XGamepad`.
- Obter o índice XInput real (`get_user_index`) para exibir "P1..P4" correto na UI.
- Notificações de rumble: registrar o callback de notificação do target (confirmar o suporte no `vigem-client`; se faltar, usar FFI mínima para `vigem_target_x360_register_notification` via `ViGEmClient` C, com `unsafe` isolado e documentado).
- Tratar a perda do driver (serviço parado) → `DriverStatus::Error` + evento para a UI.

## Detalhes técnicos
- `update()` é chamado no thread de input; medir o custo (esperado < 100 µs).
- Remoção limpa ao fechar o app (Drop + handler de encerramento), para não deixar controles "fantasma".

## Critérios de aceite
- [ ] Teste manual: o controle aparece em `joy.cpl` e no Gamepad Tester; todos os botões, gatilhos e analógicos respondem.
- [ ] Rumble do "XInput Test" chega como `RumbleEvent`.
- [ ] Fechar o app remove os controles.
- [ ] Sem ViGEmBus instalado → `status() == NotInstalled` (sem pânico).

## Fora de escopo
- Instalação do driver (T-205). Gestão de slots (T-203).
