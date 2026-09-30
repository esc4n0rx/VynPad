# T-506 — Bridge tipada Tauri ↔ React

| Campo | Valor |
|---|---|
| Fase | F5 — UI Shell |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-002 |
| Status | 🟨 Pronta; falta rodar no app real (janela) e o `useMetrics` (T-305/T-606) |

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
- [x] Mudar a assinatura de um command em Rust quebra o `pnpm typecheck`.
- [x] Sem nenhuma tela assinando, o backend não emite `input:live`.

## Fora de escopo
- Commands específicos de cada domínio (nas respectivas tasks).

## Situação

**Backend** (`apps/desktop/src-tauri`):
- `tauri-specta 2.0.0-rc.25` + `specta 2.0.0-rc.25` + `specta-typescript` (só existem release candidates; as versões são fixadas com `=`). `Builder` em `lib.rs` com `error_handling(Throw)` (os commands lançam o `AppError` no TS, o que combina com o React Query) e `dangerously_cast_bigints_to_number` (timestamps em epoch ms cabem em `number`).
- `bindings.ts` é gerado em debug ao iniciar o app **e** por `UPDATE_BINDINGS=1 cargo test -p padgaming-desktop bindings_ts`. O teste `bindings_ts_esta_em_dia` falha no CI se o arquivo versionado não bate com o gerador. Verificado: mudar a assinatura de `rename_device` e regenerar quebra o `pnpm typecheck` (`Expected 3 arguments, but got 2`).
- `AppState { core: Arc<Core>, live }`: o `Core` sobe no `setup` (pasta de dados `%APPDATA%\PadGaming`) e é encerrado (`Core::stop`, com `bye{shutdown}` e goodbye do mDNS) em `RunEvent::Exit`.
- 33 commands (os da T-403 e os do server: slots, pareamento, descoberta, interfaces, input ao vivo) e 14 eventos tipados (`SettingsChanged`, `SlotsChanged`, `DevicesChanged`, `DevicesPaired`, `DevicesDiscovered`, `SessionStarted/Ended/State`, `ConnectionRejected`, `ReservationExpired`, `ApprovalRequested/Resolved`, `DriverChanged`, `InputLive`). Os tipos que cruzam a ponte são DTOs do app; o perfil e as configurações usam o `specta::Type` opcional dos crates `mapping` e `storage` (feature `specta`).
- `AppError` com `code` estável (`internal`, `invalid_input`, `unsupported`, `not_found`, `builtin_profile`, `profile_in_use` com `in_use`, `import_invalid`, `no_network`) e `message_key`.
- **`input:live` sob demanda** (`live.rs`): `subscribe_live_input`/`unsubscribe_live_input` contam assinaturas por slot; a tarefa dorme sem assinantes e, com alguma, lê a ~60 Hz o último estado do pad (`SlotManager::live_report`, duas escritas atômicas no hot path) e só emite o que mudou. `reset_live_input` zera a contagem ao recarregar a janela.
- Testes: 29 no crate (commands pela camada IPC do Tauri com o `Core` de verdade, tradução de eventos, `LiveInput` com tempo simulado, erros, e o `bindings.ts` em dia).

**Frontend** (`apps/desktop/src`):
- `lib/api/*`: hooks do React Query por domínio (`useSettings`, `useProfiles`, `useDevices`, `useServerInfo`, ...), stores `zustand` (`useSlots`, `useLiveInput`, `useApprovals`) e `registerEventListeners` (uma vez no boot; ligado no `App`), que mantém cache e stores em dia pelos eventos.
- `useLiveReport(slot)` assina enquanto o componente está montado (contagem no JS: o backend só vê 0→1 e 1→0; seguro com StrictMode).
- Todo erro de command vira aviso (`Toaster`) com a mensagem traduzida (`i18n/errors.ts`, provisório até a T-504); mutações que tratam o erro sozinhas (`useDeleteProfile`, que pede confirmação com os números de `in_use`) usam `meta.handlesErrors`.
- Vitest + Testing Library (28 testes, `pnpm test`, também no CI): eventos, hooks, assinatura do input ao vivo, mensagens de erro e avisos, sobre um dublê dos bindings gerado a partir do próprio `bindings.ts`.

Pendências:
- `useMetrics` (histograma de latência) chega com a T-305/T-606; o backend ainda não emite métricas.
- Não vi o app numa janela real nesta sessão: os testes cobrem a camada IPC do Tauri e os hooks, não o WebView.
- O rumble (T-204) e o `config` de `rumble_enabled` seguem pendentes.
- `deny.toml`: adicionei a exceção `RUSTSEC-2024-0436` (`paste`, sem manutenção, só macros de compilação) que chega pelo `specta`.
