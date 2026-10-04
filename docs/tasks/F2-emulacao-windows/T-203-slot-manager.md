# T-203 — Slot Manager (4 controles)

| Campo | Valor |
|---|---|
| Fase | F2 — Emulação Windows |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-202 |
| Status | ✅ Concluída |

## Objetivo
Gerenciar até 4 controles virtuais simultâneos, cada um vinculado a uma sessão, com regras previsíveis de atribuição.

## Escopo
- Estados do slot: `Free` → `Reserved(session)` → `Active(session, pad)` → `Suspended(session, since)` → `Free`.
- Atribuição: 1) slot preferido do dispositivo (se livre); 2) menor slot livre; 3) `reject{no_slot}`.
- Plugar o pad só quando chega o primeiro input (evita controle fantasma se o handshake falhar).
- Ações da UI: trocar dois slots de lugar, desconectar um slot, "pausar" (entrada ignorada, pad neutro).
- Publicar `SlotSnapshot` (sem locks no hot path: `ArcSwap` ou canal) para a UI e para as métricas.
- Estado neutro garantido ao sair de `Active`.

## Detalhes técnicos
- Estrutura `[SlotState; 4]`; o thread de input acessa só o `PadHandle` e o perfil compilado do slot.
- A troca de slots exige unplug/replug no ViGEm (o índice XInput é atribuído pelo Windows); documentar essa limitação na UI.

## Critérios de aceite
- [x] Testes com `MockDriver` cobrindo todas as transições.
- [x] O 5º client recebe `no_slot`.
- [x] Reconexão dentro de 30 s volta para o mesmo slot.

## Fora de escopo
- Timeouts de rede (T-304).

## Implementação
- `vynpad-core::slots::SlotManager` (sobre `Arc<dyn VirtualPadDriver>`), com `Clock` injetável (`vynpad-core::clock`).
- API de rede: `assign` (preferido → menor livre → `NoSlot`; reconexão do mesmo dispositivo volta ao mesmo slot sem replugar), `submit` (hot path), `suspend`, `release`, `tick` (expira reservas de 30 s).
- API da UI: `disconnect_slot` (devolve a sessão para o `bye{kicked}`), `set_paused`, `swap`, `snapshot` e `subscribe` (uma `SlotsSnapshot` por mudança).
- Hot path: tabela `ArcSwap` de controles ativos; `submit` não usa lock nem aloca (`tests/hot_path.rs`), exceto no primeiro input da sessão, que pluga o controle.
- Decisão: `swap` remove e repluga os controles ativos em ordem de slot; se o replug falhar, o slot volta a `Reserved` e pluga no próximo input.
- Fica para a T-302/T-304: chamar `assign`/`suspend`/`release`/`tick` a partir do servidor de controle.
