//! Abertura do banco: pool, migrations no boot, backup e restauração.

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Row, SqlitePool};

use crate::backup;
use crate::error::{MigrationFailure, StorageError};

/// Nome do arquivo do banco.
pub const DB_FILE: &str = "padgaming.db";
/// Conexões do pool. No modo WAL há um escritor e vários leitores.
const MAX_CONNECTIONS: u32 = 5;
/// Quanto uma conexão espera o lock do banco antes de falhar.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// As migrations embutidas no binário (`migrations/*.sql`). O sqlx confere o checksum de cada
/// uma: **nunca edite uma migration já publicada**, crie uma nova.
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Pasta de dados do app: `%APPDATA%\PadGaming` no Windows, `~/.local/share/padgaming` no Linux
/// (ARQUITETURA §Dados). É onde ficam `padgaming.db` e a identidade TLS.
pub fn default_data_dir() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "PadGaming"
    } else {
        "padgaming"
    };
    dirs::data_dir().map(|dir| dir.join(name))
}

/// O banco aberto e já migrado.
#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
    path: Option<PathBuf>,
}

impl Database {
    /// Abre (ou cria) `padgaming.db` em `dir` com as migrations embutidas.
    pub async fn open(dir: &Path) -> Result<Self, StorageError> {
        Self::open_with(dir, &MIGRATOR).await
    }

    /// Como [`Self::open`], com outro conjunto de migrations (testes).
    ///
    /// Se há migrations pendentes e o banco já tem dados, faz antes um backup
    /// (`padgaming.db.bak-<versão>`, mantendo os 3 últimos). Se a migração falhar, o banco volta
    /// ao backup e o erro é uma mensagem para o usuário ([`StorageError::Migration`]).
    pub async fn open_with(dir: &Path, migrator: &Migrator) -> Result<Self, StorageError> {
        std::fs::create_dir_all(dir).map_err(|e| StorageError::io(dir, e))?;
        let path = dir.join(DB_FILE);
        let pool = connect(&path).await?;
        match migrate_file(&pool, &path, migrator).await {
            Ok(()) => {
                tracing::info!(path = %path.display(), "banco de dados pronto");
                Ok(Self {
                    pool,
                    path: Some(path),
                })
            }
            Err(error) => {
                pool.close().await;
                Err(error)
            }
        }
    }

    /// Banco em memória, com as migrations embutidas (testes).
    pub async fn in_memory() -> Result<Self, StorageError> {
        Self::in_memory_with(&MIGRATOR).await
    }

    /// Banco em memória com outro conjunto de migrations.
    pub async fn in_memory_with(migrator: &Migrator) -> Result<Self, StorageError> {
        // Cada conexão a `:memory:` seria um banco diferente: uma só conexão, sem expirar.
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .min_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await?;
        migrator.run(&pool).await.map_err(|error| {
            StorageError::Migration(Box::new(failure(migrator, &error, None, false, false)))
        })?;
        Ok(Self { pool, path: None })
    }

    /// O pool de conexões.
    pub const fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// O arquivo do banco (`None` em memória).
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Fecha as conexões (e faz o checkpoint do WAL).
    pub async fn close(self) {
        self.pool.close().await;
    }
}

async fn connect(path: &Path) -> Result<SqlitePool, StorageError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(BUSY_TIMEOUT);
    Ok(SqlitePoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .connect_with(options)
        .await?)
}

/// Versões já aplicadas com sucesso (vazio se a tabela de controle ainda não existe).
async fn applied_versions(pool: &SqlitePool) -> Result<Vec<i64>, sqlx::Error> {
    let has_table: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_optional(pool)
    .await?;
    if has_table.is_none() {
        return Ok(Vec::new());
    }
    sqlx::query("SELECT version FROM _sqlx_migrations WHERE success = 1")
        .fetch_all(pool)
        .await?
        .iter()
        .map(|row| row.try_get::<i64, _>("version"))
        .collect()
}

fn failure(
    migrator: &Migrator,
    error: &MigrateError,
    backup: Option<PathBuf>,
    restored: bool,
    discarded_fresh: bool,
) -> MigrationFailure {
    let version = match error {
        MigrateError::ExecuteMigration(_, v)
        | MigrateError::VersionMissing(v)
        | MigrateError::VersionMismatch(v)
        | MigrateError::Dirty(v) => Some(*v),
        _ => None,
    };
    let description = version.and_then(|v| {
        migrator
            .iter()
            .find(|m| m.version == v)
            .map(|m| m.description.to_string())
    });
    MigrationFailure {
        version,
        description,
        cause: error.to_string(),
        backup,
        restored,
        discarded_fresh,
    }
}

async fn migrate_file(
    pool: &SqlitePool,
    path: &Path,
    migrator: &Migrator,
) -> Result<(), StorageError> {
    let applied = applied_versions(pool).await?;
    let latest_known = migrator.iter().map(|m| m.version).max().unwrap_or(0);
    let newest_applied = applied.iter().copied().max().unwrap_or(0);
    if newest_applied > latest_known && !migrator.ignore_missing {
        return Err(StorageError::TooNew {
            found: newest_applied,
            latest: latest_known,
        });
    }
    if migrator.iter().all(|m| applied.contains(&m.version)) {
        return Ok(());
    }

    // Só há o que proteger se o banco já tem migrations aplicadas.
    let has_data = !applied.is_empty();
    let backup = if has_data {
        let made = backup::create(pool, path, newest_applied).await?;
        tracing::info!(backup = %made.display(), "backup do banco criado antes de migrar");
        Some(made)
    } else {
        None
    };

    match migrator.run(pool).await {
        Ok(()) => {
            backup::prune(path);
            Ok(())
        }
        Err(error) => {
            tracing::error!(%error, "a migration falhou");
            // As conexões precisam fechar antes de trocar o arquivo.
            pool.close().await;
            let (restored, discarded_fresh) = if let Some(saved) = &backup {
                match backup::restore(path, saved) {
                    Ok(()) => (true, false),
                    Err(restore_error) => {
                        tracing::error!(%restore_error, "a restauração do backup falhou");
                        (false, false)
                    }
                }
            } else {
                backup::discard(path);
                (false, true)
            };
            Err(StorageError::Migration(Box::new(failure(
                migrator,
                &error,
                backup,
                restored,
                discarded_fresh,
            ))))
        }
    }
}
