//! Persistência em SQLite: pool, migrations e (T-403) repositórios.
//!
//! - [`Database`]: abre `padgaming.db` no diretório de dados com WAL, `synchronous=NORMAL`,
//!   `foreign_keys=ON` e `busy_timeout=5s`, e roda as migrations embutidas ([`MIGRATOR`]) no boot.
//! - Antes de migrar um banco que já tem dados, faz o backup `padgaming.db.bak-<versão>` (os 3
//!   últimos ficam); se a migração falhar, restaura o backup e devolve um erro legível
//!   ([`StorageError::Migration`]).
//! - [`Database::in_memory`] cria um banco em memória para os testes.
//!
//! # Migrations
//!
//! Ficam em `crates/padgaming-storage/migrations/<versão>_<descrição>.sql` (`0001_schema.sql`...)
//! e são embutidas por `sqlx::migrate!`. Regras:
//! - nunca edite uma migration já publicada (o sqlx confere o checksum e o boot falharia);
//! - o banco é só do backend Rust: a UI nunca fala com o SQLite (D-07).
//!
//! # Queries verificadas em compilação
//!
//! O CI não tem banco, então as macros `query!`/`query_as!` (T-403) usam o modo offline: o
//! `SQLX_OFFLINE=true` em `.cargo/config.toml` faz o compilador ler os metadados de `.sqlx/`, que
//! são versionados. Para atualizá-los depois de mudar uma query ou uma migration, veja o README
//! do crate.

pub mod backup;
mod db;
mod error;

pub use db::{DB_FILE, Database, MIGRATOR, default_data_dir};
pub use error::{MigrationFailure, StorageError};
pub use sqlx::SqlitePool;
