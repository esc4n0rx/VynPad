# padgaming-storage

Persistência em SQLite (`sqlx`): pool, migrations embutidas, backup e restauração, schema v1
(`settings`, `profiles`, `devices`, `connection_log`), seeds e retenção do histórico. Os
repositórios chegam na T-403.

- Banco: `%APPDATA%\PadGaming\padgaming.db` (Windows) ou `~/.local/share/padgaming/padgaming.db`
  (Linux); ver `default_data_dir()`.
- Conexão: WAL, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5s`.
- Migrations: `migrations/<versão>_<descrição>.sql` (`0001_schema.sql`...), embutidas no binário e
  aplicadas no boot. **Nunca edite uma migration publicada**: o sqlx confere o checksum.
- Antes de migrar um banco com dados, `padgaming.db.bak-<versão>` (ficam os 3 últimos). Se a
  migration falhar, o banco volta ao backup e o erro é uma mensagem para o usuário.
- Testes: `Database::in_memory()`.

## Queries verificadas em compilação

O `.cargo/config.toml` define `SQLX_OFFLINE=true`: `query!`/`query_as!` leem os metadados de
`.sqlx/` (versionado), então o CI compila sem banco nem `DATABASE_URL`. Depois de mudar uma query
ou uma migration, regenere:

```text
cargo install sqlx-cli --no-default-features --features sqlite,rustls   # uma vez
# bash / Git Bash
export DATABASE_URL=sqlite://target/prepare.db SQLX_OFFLINE=false
cargo sqlx database create && cargo sqlx migrate run --source crates/padgaming-storage/migrations
cargo sqlx prepare --workspace -- --all-targets
git add .sqlx
```
