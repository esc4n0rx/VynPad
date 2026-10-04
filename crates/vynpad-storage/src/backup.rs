//! Backup do banco antes de migrar, restauração e rotação (`vynpad.db.bak-<versão>`).

use std::fs;
use std::path::{Path, PathBuf};

use sqlx::{AssertSqlSafe, SqlitePool};

use crate::error::StorageError;

/// Quantos backups ficam no disco.
pub const KEEP_BACKUPS: usize = 3;

/// `vynpad.db.bak-<versão>` ao lado do banco.
pub fn backup_path(db: &Path, version: i64) -> PathBuf {
    let mut name = db.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".bak-{version}"));
    db.with_file_name(name)
}

/// Copia o banco para `vynpad.db.bak-<versão>` com um snapshot consistente (`VACUUM INTO`,
/// que já considera o que está no WAL). Sobrescreve um backup antigo da mesma versão.
pub async fn create(pool: &SqlitePool, db: &Path, version: i64) -> Result<PathBuf, StorageError> {
    let target = backup_path(db, version);
    if target.exists() {
        fs::remove_file(&target).map_err(|e| StorageError::io(&target, e))?;
    }
    // `VACUUM INTO` não aceita parâmetro. O caminho é montado por nós a partir da pasta de dados
    // e do número da versão, e aspas simples são duplicadas: não há entrada externa aqui.
    let quoted = target.to_string_lossy().replace('\'', "''");
    sqlx::query(AssertSqlSafe(format!("VACUUM INTO '{quoted}'")))
        .execute(pool)
        .await?;
    Ok(target)
}

/// Remove o `-wal` e o `-shm` do banco (com o pool fechado eles não valem mais).
fn remove_sidecars(db: &Path) {
    for suffix in ["-wal", "-shm"] {
        let mut name = db.file_name().unwrap_or_default().to_os_string();
        name.push(suffix);
        let _ = fs::remove_file(db.with_file_name(name));
    }
}

/// Devolve o banco ao estado do `backup`. O pool precisa estar fechado.
pub fn restore(db: &Path, backup: &Path) -> Result<(), StorageError> {
    remove_sidecars(db);
    fs::copy(backup, db).map_err(|e| StorageError::io(backup, e))?;
    Ok(())
}

/// Apaga o banco e os arquivos auxiliares (banco novo cuja primeira migração falhou).
pub fn discard(db: &Path) {
    remove_sidecars(db);
    let _ = fs::remove_file(db);
}

/// Mantém só os [`KEEP_BACKUPS`] backups de versão mais alta.
pub fn prune(db: &Path) {
    let Some(dir) = db.parent() else { return };
    let prefix = format!(
        "{}.bak-",
        db.file_name().unwrap_or_default().to_string_lossy()
    );
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut backups: Vec<(i64, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let version = name.strip_prefix(&prefix)?.parse::<i64>().ok()?;
            Some((version, entry.path()))
        })
        .collect();
    backups.sort_by_key(|(version, _)| std::cmp::Reverse(*version));
    for (_, path) in backups.into_iter().skip(KEEP_BACKUPS) {
        if let Err(error) = fs::remove_file(&path) {
            tracing::warn!(%error, path = %path.display(), "não consegui apagar um backup antigo");
        }
    }
}
