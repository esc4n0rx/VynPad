# T-202 — Backend ViGEmBus

| Campo | Valor |
|---|---|
| Fase | F2 — Emulação Windows |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-201 |
| Status | ✅ Concluída |

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
- [x] Todos os botões, gatilhos e analógicos chegam ao XInput (teste automatizado `report_reaches_xinput...`; conferir também no `joy.cpl`/Gamepad Tester).
- [x] Rumble enviado via XInput chega como `RumbleEvent` (`rumble_from_a_game...`).
- [x] Desligar o controle ou destruir o driver remove os controles (`dropping_the_driver...`).
- [x] Sem ViGEmBus instalado → `status() == NotInstalled` (sem pânico).

## Fora de escopo
- Instalação do driver (T-205). Gestão de slots (T-203).

## Implementação
- `vynpad-driver::VigemDriver` (só Windows) sobre `vigem-client` 0.1.4 com a feature `unstable_xtarget_notification`: a vibração já vem da própria crate, então **não há FFI nem `unsafe`**.
- `plug()` espera o controle aceitar um report neutro (o primeiro `update` logo após `wait_ready` falha com `ERROR_DEVICE_NOT_CONNECTED`).
- Testes com o driver real: `cargo test -p vynpad-driver --test vigem_hardware -- --ignored --test-threads=1`.
- Medido: `update()` ≈ 15 µs por chamada.
- `get_user_index()` do driver é pouco confiável (devolveu 0 enquanto o XInput expôs o controle no índice 2, com controle físico na máquina). Detectar o índice real fica para a T-205.
- Falha do driver durante o uso vira `DriverStatus::Error` em `status()`; o evento para a UI depende do Slot Manager (T-203).
