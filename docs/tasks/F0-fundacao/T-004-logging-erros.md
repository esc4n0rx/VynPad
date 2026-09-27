# T-004 — Logging, erros e diagnóstico

| Campo | Valor |
|---|---|
| Fase | F0 — Fundação |
| Prioridade | P0 |
| Estimativa | P |
| Depende de | T-001 |
| Status | ✅ Concluída (2026-09-26) |

## Objetivo
Padronizar erros e logs em todos os crates para facilitar o suporte e a depuração de problemas de rede e driver.

## Escopo
- `thiserror` nos crates de biblioteca; `anyhow` só nos binários/app.
- `tracing` + `tracing-subscriber` (env-filter) + `tracing-appender` (rotação diária, manter 7 arquivos).
- Níveis padrão: `info` no app, `warn` nas deps; `RUST_LOG` sobrescreve.
- Spans por sessão (`session_id`, `slot`, `device_id` truncado).
- Erros serializáveis para o frontend: `AppError { code: string, message_key: string, details?: string }`; `message_key` aponta para o i18n.
- Hook de pânico que registra no log antes de encerrar.
- **Nunca** logar tokens, chaves ou PIN (criar o tipo `Secret<T>` com `Debug` mascarado).

## Detalhes técnicos
- **Conflito resolvido:** o `tauri-plugin-log` (T-002) foi removido e o `tracing` é o sistema único de logs (D-19). Os logs do crate `log` usados pelo Tauri chegam via a ponte `tracing-log`, e os do frontend pelo command `log_from_frontend`.
- O hot path (T-301) não loga por pacote; usa contadores atômicos e loga resumos.

## Critérios de aceite
- [x] Logs gravados no diretório documentado em ARQUITETURA §6.
- [x] `format!("{:?}", secret)` resulta em `Secret(***)`.
- [x] Um erro de command chega ao React com `code` e `message_key`.

## Fora de escopo
- Pacote de diagnóstico exportável (T-705).

## Notas de implementação
- `src-tauri/src/logging.rs`: filtro padrão `warn,padgaming=info,frontend=info` (sobrescrito por `RUST_LOG`); arquivo com rotação diária, 7 arquivos mantidos; saída também no terminal.
- A escrita no arquivo é **síncrona** de propósito: o volume é baixo e garante que a mensagem de pânico chegue ao disco antes do `panic = "abort"` do release.
- Os nomes dos arquivos e os timestamps usam **UTC** (ex.: `padgaming.2026-09-27.log`).
- `src-tauri/src/error.rs`: `AppError { code, message_key, details? }`; qualquer `std::error::Error` convertido vira `internal` e é logado com a causa.
- Frontend: `src/lib/errors.ts` (tipo + `isAppError`) e `src/lib/log.ts` (`log.info/warn/error/debug` e captura de `error`/`unhandledrejection`). O backend recusa mensagens acima de 8 KB (`invalid_input`).
- `Secret<T>` fica em `padgaming-protocol::secret`, o crate mais baixo da árvore.
- **Testes no Windows:** o `build.rs` embute o manifest do Windows em todos os alvos. Sem isso, os binários de teste que usam o Tauri falham com `STATUS_ENTRYPOINT_NOT_FOUND`.
- Convenção: `thiserror` nos crates de biblioteca e `anyhow` só em binários. A dependência entra em cada crate quando ele ganhar o primeiro erro.
