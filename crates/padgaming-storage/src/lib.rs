//! Persistência em SQLite: pool, migrations e (T-403) repositórios.
//!
//! - [`Database`]: abre `padgaming.db` no diretório de dados com WAL, `synchronous=NORMAL`,
//!   `foreign_keys=ON` e `busy_timeout=5s`, e roda as migrations embutidas ([`MIGRATOR`]) no boot.
//! - Antes de migrar um banco que já tem dados, faz o backup `padgaming.db.bak-<versão>` (os 3
//!   últimos ficam); se a migração falhar, restaura o backup e devolve um erro legível
//!   ([`StorageError::Migration`]).
//! - Schema v1 (`migrations/0001_schema.sql`): `settings`, `profiles`, `devices` e
//!   `connection_log`. Os seeds (perfil "Padrão" e configurações padrão) e a retenção do histórico
//!   (30 dias / 10 mil linhas) rodam em todo boot, em [`seed`].
//! - Repositórios e serviços sobre o schema: [`SettingsService`] (`AppSettings` tipada e o aviso
//!   `settings:changed`), [`ProfileService`] (CRUD, duplicar, `.padprofile`, troca a quente via
//!   [`ProfileHost`]), [`DeviceRepo`] e [`ConnectionLog`]. As queries são verificadas em compilação.
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
//! O CI não tem banco, então as macros `query!`/`query_as!` usam o modo offline: o
//! `SQLX_OFFLINE=true` em `.cargo/config.toml` faz o compilador ler os metadados de `.sqlx/`, que
//! são versionados. Para atualizá-los depois de mudar uma query ou uma migration, rode
//! `scripts/sqlx-prepare.sh` (veja o README do crate).

pub mod backup;
mod db;
pub mod devices;
mod error;
pub mod log;
pub mod profiles;
pub mod seed;
pub mod settings;

pub use db::{DB_FILE, Database, MIGRATOR, default_data_dir};
pub use devices::{Device, DeviceDefaults, DeviceRepo, NewPairing};
pub use error::{DataError, MigrationFailure, StorageError};
pub use log::{ConnectionLog, LogEntry, LogEvent, LogQuery};
pub use profiles::{
    NoHost, ProfileHost, ProfileInput, ProfileRepo, ProfileService, ProfileSummary,
    ResolvedProfile, StoredProfile,
};
pub use settings::{AppSettings, Language, Ports, SettingsRepo, SettingsService, Theme};
pub use sqlx::SqlitePool;
