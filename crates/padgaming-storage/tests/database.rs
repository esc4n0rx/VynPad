//! Abertura, migrations, backup e restauração do banco (arquivo real em pasta temporária).

use std::borrow::Cow;
use std::path::Path;

use padgaming_storage::{DB_FILE, Database, StorageError, backup, default_data_dir};
use sqlx::migrate::{Migration, MigrationType, Migrator};
use sqlx::{Row, SqlSafeStr};

fn migration(version: i64, description: &'static str, sql: &'static str) -> Migration {
    Migration::new(
        version,
        Cow::Borrowed(description),
        MigrationType::Simple,
        sql.into_sql_str(),
        false,
    )
}

const V1: &str = "CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT NOT NULL);";
const V2: &str = "ALTER TABLE notes ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;";
const V3: &str =
    "CREATE TABLE tags (id INTEGER PRIMARY KEY, note_id INTEGER NOT NULL REFERENCES notes(id));";
const V4: &str = "CREATE INDEX idx_notes_body ON notes(body);";
const V5: &str = "CREATE INDEX idx_notes_pinned ON notes(pinned);";
/// A primeira instrução vale; a segunda falha (tabela inexistente).
const BROKEN: &str = "CREATE TABLE halfway (id INTEGER); INSERT INTO nao_existe VALUES (1);";

fn v1() -> Migrator {
    Migrator::with_migrations(vec![migration(1, "notes", V1)])
}

fn v2() -> Migrator {
    Migrator::with_migrations(vec![migration(1, "notes", V1), migration(2, "pinned", V2)])
}

async fn scalar(db: &Database, sql: &'static str) -> i64 {
    sqlx::query(sql)
        .fetch_one(db.pool())
        .await
        .unwrap()
        .get::<i64, _>(0)
}

async fn table_exists(db: &Database, name: &str) -> bool {
    sqlx::query("SELECT count(*) FROM sqlite_master WHERE type='table' AND name = ?")
        .bind(name)
        .fetch_one(db.pool())
        .await
        .unwrap()
        .get::<i64, _>(0)
        == 1
}

fn backups(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".bak-"))
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn first_boot_creates_the_database_file_in_the_given_folder() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("dados").join("PadGaming"); // a pasta nem existe ainda
    let db = Database::open_with(&dir, &v1()).await.unwrap();
    assert_eq!(db.path(), Some(dir.join(DB_FILE).as_path()));
    assert!(dir.join(DB_FILE).is_file());
    assert!(table_exists(&db, "notes").await);
    // Banco novo: nada a proteger, então nenhum backup.
    assert!(backups(&dir).is_empty());
}

#[tokio::test]
async fn the_embedded_migrations_open_a_fresh_database() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).await.unwrap();
    db.close().await;
    // Reabrir é idempotente.
    Database::open(dir.path()).await.unwrap().close().await;
}

#[tokio::test]
async fn the_connection_pragmas_are_the_documented_ones() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_with(dir.path(), &v1()).await.unwrap();
    let journal: String = sqlx::query("PRAGMA journal_mode")
        .fetch_one(db.pool())
        .await
        .unwrap()
        .get(0);
    assert_eq!(journal.to_lowercase(), "wal");
    assert_eq!(scalar(&db, "PRAGMA synchronous").await, 1, "NORMAL");
    assert_eq!(scalar(&db, "PRAGMA foreign_keys").await, 1);
    assert_eq!(scalar(&db, "PRAGMA busy_timeout").await, 5000);
}

#[tokio::test]
async fn foreign_keys_are_enforced_on_every_connection() {
    let dir = tempfile::tempdir().unwrap();
    let migrator = Migrator::with_migrations(vec![
        migration(1, "notes", V1),
        migration(2, "pinned", V2),
        migration(3, "tags", V3),
    ]);
    let db = Database::open_with(dir.path(), &migrator).await.unwrap();
    // Várias conexões ao mesmo tempo: nenhuma pode aceitar uma nota inexistente.
    let attempts = (0..5).map(|_| {
        let pool = db.pool().clone();
        tokio::spawn(async move {
            sqlx::query("INSERT INTO tags (note_id) VALUES (999)")
                .execute(&pool)
                .await
        })
    });
    for attempt in attempts {
        assert!(attempt.await.unwrap().is_err());
    }
}

#[tokio::test]
async fn upgrading_backs_up_first_and_keeps_the_data() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_with(dir.path(), &v1()).await.unwrap();
    sqlx::query("INSERT INTO notes (body) VALUES ('antes')")
        .execute(db.pool())
        .await
        .unwrap();
    db.close().await;

    let db = Database::open_with(dir.path(), &v2()).await.unwrap();
    assert_eq!(
        scalar(&db, "SELECT count(*) FROM notes WHERE pinned = 0").await,
        1
    );
    assert_eq!(backups(dir.path()), ["padgaming.db.bak-1"]);

    // O backup é o banco de antes da migration: tem a nota e não tem a coluna nova.
    let old = Database::open_with(
        &{
            let copy = tempfile::tempdir().unwrap();
            std::fs::copy(
                dir.path().join("padgaming.db.bak-1"),
                copy.path().join(DB_FILE),
            )
            .unwrap();
            copy.keep()
        },
        &v1(),
    )
    .await
    .unwrap();
    assert_eq!(scalar(&old, "SELECT count(*) FROM notes").await, 1);
    assert!(
        sqlx::query("SELECT pinned FROM notes")
            .fetch_all(old.pool())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn reopening_without_pending_migrations_makes_no_backup() {
    let dir = tempfile::tempdir().unwrap();
    Database::open_with(dir.path(), &v2())
        .await
        .unwrap()
        .close()
        .await;
    Database::open_with(dir.path(), &v2())
        .await
        .unwrap()
        .close()
        .await;
    assert!(backups(dir.path()).is_empty());
}

#[tokio::test]
async fn a_failing_migration_restores_the_backup_and_explains_itself() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_with(dir.path(), &v1()).await.unwrap();
    sqlx::query("INSERT INTO notes (body) VALUES ('preciosa')")
        .execute(db.pool())
        .await
        .unwrap();
    db.close().await;

    let broken = Migrator::with_migrations(vec![
        migration(1, "notes", V1),
        migration(2, "quebrada", BROKEN),
    ]);
    let error = Database::open_with(dir.path(), &broken).await.unwrap_err();
    let StorageError::Migration(failure) = &error else {
        panic!("esperava StorageError::Migration, veio {error:?}");
    };
    assert_eq!(failure.version, Some(2));
    assert_eq!(failure.description.as_deref(), Some("quebrada"));
    assert!(failure.restored);
    assert_eq!(
        failure.backup.as_deref(),
        Some(dir.path().join("padgaming.db.bak-1").as_path())
    );
    let message = error.user_message();
    assert!(message.contains("etapa 2: quebrada"), "{message}");
    assert!(message.contains("restaurados do backup"), "{message}");
    assert!(
        message.contains("nao_existe"),
        "o detalhe técnico deve vir junto: {message}"
    );

    // O banco voltou ao que era: dado preservado, migration 2 não deixou rastro (nem a primeira
    // instrução da migration quebrada) e a versão registrada continua a 1.
    let db = Database::open_with(dir.path(), &v1()).await.unwrap();
    assert_eq!(scalar(&db, "SELECT count(*) FROM notes").await, 1);
    assert!(!table_exists(&db, "halfway").await);
    assert_eq!(
        scalar(&db, "SELECT max(version) FROM _sqlx_migrations").await,
        1
    );
}

#[tokio::test]
async fn the_app_can_boot_again_after_a_restored_failure() {
    let dir = tempfile::tempdir().unwrap();
    Database::open_with(dir.path(), &v1())
        .await
        .unwrap()
        .close()
        .await;
    let broken = Migrator::with_migrations(vec![
        migration(1, "notes", V1),
        migration(2, "quebrada", BROKEN),
    ]);
    assert!(Database::open_with(dir.path(), &broken).await.is_err());
    // A versão corrigida do app migra normalmente em cima do banco restaurado.
    let db = Database::open_with(dir.path(), &v2()).await.unwrap();
    assert_eq!(
        scalar(&db, "SELECT max(version) FROM _sqlx_migrations").await,
        2
    );
}

#[tokio::test]
async fn a_failure_on_a_brand_new_database_discards_it() {
    let dir = tempfile::tempdir().unwrap();
    let broken = Migrator::with_migrations(vec![migration(1, "quebrada", BROKEN)]);
    let error = Database::open_with(dir.path(), &broken).await.unwrap_err();
    let StorageError::Migration(failure) = &error else {
        panic!("{error:?}");
    };
    assert!(failure.discarded_fresh && !failure.restored && failure.backup.is_none());
    assert!(error.user_message().contains("recomeça do zero"));
    assert!(!dir.path().join(DB_FILE).exists());
    // O próximo boot, já com a migration certa, funciona.
    Database::open_with(dir.path(), &v1())
        .await
        .unwrap()
        .close()
        .await;
}

#[tokio::test]
async fn only_the_three_latest_backups_are_kept() {
    let dir = tempfile::tempdir().unwrap();
    let steps = [
        migration(1, "notes", V1),
        migration(2, "pinned", V2),
        migration(3, "tags", V3),
        migration(4, "index", V4),
        migration(5, "index2", V5),
    ];
    // Cada atualização (1→2, 2→3, 3→4, 4→5) deixa o backup da versão de que partiu.
    for upto in 1..=5 {
        let migrator = Migrator::with_migrations(steps[..upto].to_vec());
        Database::open_with(dir.path(), &migrator)
            .await
            .unwrap()
            .close()
            .await;
    }
    assert_eq!(
        backups(dir.path()),
        [
            "padgaming.db.bak-2",
            "padgaming.db.bak-3",
            "padgaming.db.bak-4"
        ]
    );
}

#[tokio::test]
async fn a_database_from_a_newer_app_is_refused_untouched() {
    let dir = tempfile::tempdir().unwrap();
    Database::open_with(dir.path(), &v2())
        .await
        .unwrap()
        .close()
        .await;
    let error = Database::open_with(dir.path(), &v1()).await.unwrap_err();
    assert!(
        matches!(
            error,
            StorageError::TooNew {
                found: 2,
                latest: 1
            }
        ),
        "{error:?}"
    );
    assert!(error.user_message().contains("versão mais nova"));
    assert!(backups(dir.path()).is_empty());
}

#[tokio::test]
async fn an_edited_migration_is_reported_in_plain_language_and_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    Database::open_with(dir.path(), &v1())
        .await
        .unwrap()
        .close()
        .await;
    let edited = Migrator::with_migrations(vec![
        migration(
            1,
            "notes",
            "CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT, extra TEXT);",
        ),
        migration(2, "pinned", V2),
    ]);
    let error = Database::open_with(dir.path(), &edited).await.unwrap_err();
    let StorageError::Migration(failure) = &error else {
        panic!("{error:?}");
    };
    assert_eq!(failure.version, Some(1));
    assert!(failure.cause.contains("modified"), "{}", failure.cause);
    // O banco continua abrindo com a migration original.
    Database::open_with(dir.path(), &v1())
        .await
        .unwrap()
        .close()
        .await;
}

#[tokio::test]
async fn the_in_memory_database_works_like_a_file_one() {
    let db = Database::in_memory_with(&v2()).await.unwrap();
    assert!(db.path().is_none());
    sqlx::query("INSERT INTO notes (body) VALUES ('x')")
        .execute(db.pool())
        .await
        .unwrap();
    // O pool reaproveita a mesma conexão: o dado continua lá em várias consultas.
    for _ in 0..3 {
        assert_eq!(scalar(&db, "SELECT count(*) FROM notes").await, 1);
    }
    assert_eq!(scalar(&db, "PRAGMA foreign_keys").await, 1);
    let embedded = Database::in_memory().await.unwrap();
    embedded.close().await;
}

#[test]
fn the_default_data_dir_matches_the_documented_location() {
    let dir = default_data_dir().expect("há um diretório de dados do usuário");
    let name = dir.file_name().unwrap().to_string_lossy().into_owned();
    if cfg!(windows) {
        assert_eq!(name, "PadGaming");
        assert!(dir.starts_with(std::env::var("APPDATA").unwrap()));
    } else {
        assert_eq!(name, "padgaming");
    }
}

#[test]
fn backup_names_follow_the_documented_pattern() {
    let path = backup::backup_path(Path::new("/dados/padgaming.db"), 7);
    assert_eq!(path, Path::new("/dados/padgaming.db.bak-7"));
}
