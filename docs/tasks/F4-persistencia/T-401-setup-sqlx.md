# T-401 — Setup SQLite + sqlx + migrations

| Campo | Valor |
|---|---|
| Fase | F4 — Persistência |
| Prioridade | P0 |
| Estimativa | P |
| Depende de | T-001 |
| Status | ⬜ A fazer |

## Objetivo
Base de persistência confiável, versionada e testável.

## Escopo
- Crate `padgaming-storage` com `sqlx` (features `sqlite`, `runtime-tokio`, `macros`, `migrate`).
- `SqlitePool` com `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5s`.
- Migrations em `migrations/` embutidas com `sqlx::migrate!()` e executadas no boot.
- Queries verificadas em tempo de compilação com o modo offline (`sqlx prepare`, `.sqlx/` versionado) para o CI funcionar sem banco.
- Backup automático antes de cada migration (`padgaming.db.bak-<versão>`), mantendo os 3 últimos.
- Banco em memória para os testes.

## Critérios de aceite
- [ ] O app cria o banco no primeiro boot, no caminho documentado.
- [ ] O CI compila sem `DATABASE_URL`.
- [ ] Uma migration que falha restaura o backup e mostra um erro amigável.

## Fora de escopo
- Tabelas de domínio (T-402).
