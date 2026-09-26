# T-004 — Logging, erros e diagnóstico

| Campo | Valor |
|---|---|
| Fase | F0 — Fundação |
| Prioridade | P0 |
| Estimativa | P |
| Depende de | T-001 |
| Status | ⬜ A fazer |

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
- O hot path (T-301) não loga por pacote; usa contadores atômicos e loga resumos.

## Critérios de aceite
- [ ] Logs gravados no diretório documentado em ARQUITETURA §6.
- [ ] `format!("{:?}", secret)` resulta em `Secret(***)`.
- [ ] Um erro de command chega ao React com `code` e `message_key`.

## Fora de escopo
- Pacote de diagnóstico exportável (T-705).
