# T-506 — Bridge tipada Tauri ↔ React

| Campo | Valor |
|---|---|
| Fase | F5 — UI Shell |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-002 |
| Status | ⬜ A fazer |

## Objetivo
Comunicação entre frontend e backend 100% tipada, sem strings mágicas.

## Escopo
- `tauri-specta` (+ `specta`) gerando `src/lib/bindings.ts` com commands **e** eventos tipados no build de dev.
- `AppState` no `src-tauri` com handle para o `padgaming-core` (`Arc<Core>`).
- Camada `src/lib/api/`: hooks React Query por domínio (`useSettings`, `useProfiles`, `useDevices`…).
- Stores `zustand` para dados de alta frequência (`useLiveInput`, `useMetrics`, `useSlots`), alimentados por listeners de eventos registrados uma única vez no boot.
- Assinatura sob demanda do `input:live`: o frontend chama `subscribe_live_input(slot)` / `unsubscribe` para o backend só emitir quando há alguém vendo.
- Erros padronizados (`AppError` da T-004) → toast automático com a mensagem traduzida.

## Critérios de aceite
- [ ] Mudar a assinatura de um command em Rust quebra o `pnpm typecheck`.
- [ ] Sem nenhuma tela assinando, o backend não emite `input:live`.

## Fora de escopo
- Commands específicos de cada domínio (nas respectivas tasks).
