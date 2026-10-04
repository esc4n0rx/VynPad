# T-401 — Setup SQLite + sqlx + migrations

| Campo | Valor |
|---|---|
| Fase | F4 — Persistência |
| Prioridade | P0 |
| Estimativa | P |
| Depende de | T-001 |
| Status | 🟨 Pronta na crate; falta abrir o banco no boot do app (T-403) e gerar `.sqlx/` (T-403) |

## Objetivo
Base de persistência confiável, versionada e testável.

## Escopo
- Crate `vynpad-storage` com `sqlx` (features `sqlite`, `runtime-tokio`, `macros`, `migrate`).
- `SqlitePool` com `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5s`.
- Migrations em `migrations/` embutidas com `sqlx::migrate!()` e executadas no boot.
- Queries verificadas em tempo de compilação com o modo offline (`sqlx prepare`, `.sqlx/` versionado) para o CI funcionar sem banco.
- Backup automático antes de cada migration (`vynpad.db.bak-<versão>`), mantendo os 3 últimos.
- Banco em memória para os testes.

## Critérios de aceite
- [x] (na crate; a chamada no boot do app vem com os commands da T-403) O app cria o banco no primeiro boot, no caminho documentado.
- [x] O CI compila sem `DATABASE_URL`.
- [x] Uma migration que falha restaura o backup e mostra um erro amigável.

## Fora de escopo
- Tabelas de domínio (T-402).

## Situação

Pronto em `vynpad-storage` (`Database`):
- `Database::open(dir)`: cria a pasta e `vynpad.db`, pool de 5 conexões com WAL, `synchronous=NORMAL`, `foreign_keys=ON` e `busy_timeout=5s` (conferidos por teste, inclusive as chaves estrangeiras em várias conexões). `default_data_dir()` devolve `%APPDATA%\VynPad` / `~/.local/share/vynpad`.
- Migrations embutidas por `sqlx::migrate!("./migrations")` (com `build.rs` para recompilar quando mudam) e aplicadas no boot. A pasta ainda está vazia: a primeira (`0001_schema.sql`) é a T-402.
- Backup `vynpad.db.bak-<versão>` (via `VACUUM INTO`, consistente com o WAL) só quando há migrations pendentes **e** o banco já tem dados; ficam os 3 de versão mais alta.
- Migration que falha: fecha o pool, restaura o backup e devolve `StorageError::Migration` com mensagem em português (etapa, se os dados foram restaurados, onde está o backup, detalhe técnico). Como o SQLite tem DDL transacional, uma migration com instrução quebrada no meio não deixa rastro. Banco novo (sem dados) cuja migration falha é descartado. Banco de versão mais nova que o app → `StorageError::TooNew`; migration editada depois de publicada → erro claro do checksum.
- `Database::in_memory()` para os testes (uma conexão só, senão cada conexão seria um banco).
- Sem `DATABASE_URL`: `.cargo/config.toml` define `SQLX_OFFLINE=true`. Ainda não há macros `query!` (não há tabelas), então `.sqlx/` passou a existir na T-403, gerado por `scripts/sqlx-prepare.sh` (sem `sqlx-cli`).

Testes (15, arquivo real em pasta temporária): primeiro boot, pragmas, atualização com backup e dados preservados, falha com restauração (e o app subindo de novo depois), banco novo com falha, rotação dos backups, banco mais novo, migration editada, memória e caminho padrão.
