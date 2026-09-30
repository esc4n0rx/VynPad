# padgaming-storage

Persistência em SQLite (`sqlx`): pool, migrations embutidas, backup e restauração, schema v1
(`settings`, `profiles`, `devices`, `connection_log`), seeds e retenção do histórico. Repositórios e
serviços (T-403).

- Banco: `%APPDATA%\PadGaming\padgaming.db` (Windows) ou `~/.local/share/padgaming/padgaming.db`
  (Linux); ver `default_data_dir()`.
- Conexão: WAL, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5s`.
- Migrations: `migrations/<versão>_<descrição>.sql` (`0001_schema.sql`...), embutidas no binário e
  aplicadas no boot. **Nunca edite uma migration publicada**: o sqlx confere o checksum.
- Antes de migrar um banco com dados, `padgaming.db.bak-<versão>` (ficam os 3 últimos). Se a
  migration falhar, o banco volta ao backup e o erro é uma mensagem para o usuário.
- Testes: `Database::in_memory()`.

## Repositórios e serviços

`SettingsService`, `ProfileService` (com `ProfileHost` para a troca a quente), `DeviceRepo` e
`ConnectionLog`; ver a documentação de cada módulo. Erros de validação vêm como
`DataError::Invalid(mensagem)`, já em português.

## Queries verificadas em compilação

O `.cargo/config.toml` define `SQLX_OFFLINE=true`: `query!`/`query_as!` leem os metadados de
`.sqlx/` (versionado), então o CI compila sem banco nem `DATABASE_URL`. Depois de mudar uma query
ou uma migration, regenere (Git Bash; não precisa do `sqlx-cli`):

```text
./scripts/sqlx-prepare.sh
git add .sqlx
```

O script cria um banco só com o schema (`padgaming-dbprep`) e compila este crate contra ele com
`SQLX_OFFLINE=false` e `SQLX_OFFLINE_DIR=.sqlx`: as macros conferem cada query no banco de
verdade e gravam os metadados. Se você mudar uma query e esquecer de regenerar, a compilação
offline falha.
