//! Schema v1, seeds idempotentes e retenção do histórico.

use std::borrow::Cow;

use padgaming_mapping::{CompiledProfile, Profile};
use padgaming_storage::seed::{
    self, BUILTIN_PROFILE_ID, BUILTIN_PROFILE_NAME, DEFAULT_SETTINGS, LOG_MAX_AGE_MS, LOG_MAX_ROWS,
};
use padgaming_storage::{DB_FILE, Database, MIGRATOR};
use sqlx::migrate::{Migration, MigrationType, Migrator};
use sqlx::{Row, SqlSafeStr, SqlitePool};

const NOW: i64 = 1_800_000_000_000;

async fn count(pool: &SqlitePool, sql: &'static str) -> i64 {
    sqlx::query(sql)
        .fetch_one(pool)
        .await
        .unwrap()
        .get::<i64, _>(0)
}

async fn setting(pool: &SqlitePool, key: &str) -> Option<String> {
    sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .unwrap()
}

async fn insert_device(pool: &SqlitePool, id: &str) {
    sqlx::query(
        "INSERT INTO devices (id, name, token_hash, paired_at) VALUES (?, 'Pixel', x'00', ?)",
    )
    .bind(id)
    .bind(NOW)
    .execute(pool)
    .await
    .unwrap();
}

/// Colunas de uma tabela: (nome, tipo, not null, chave primária).
async fn columns(pool: &SqlitePool, table: &'static str) -> Vec<(String, String, bool, bool)> {
    let sql = format!("SELECT name, type, \"notnull\", pk FROM pragma_table_info('{table}')");
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .fetch_all(pool)
        .await
        .unwrap()
        .iter()
        .map(|r| {
            (
                r.get::<String, _>(0),
                r.get::<String, _>(1),
                r.get::<i64, _>(2) == 1,
                r.get::<i64, _>(3) > 0,
            )
        })
        .collect()
}

fn col(name: &str, ty: &str, not_null: bool, pk: bool) -> (String, String, bool, bool) {
    (name.to_owned(), ty.to_owned(), not_null, pk)
}

#[tokio::test]
async fn the_first_migration_creates_exactly_the_documented_tables() {
    let db = Database::in_memory().await.unwrap();
    let pool = db.pool();

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE '\\_%' ESCAPE '\\' \
         AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(
        tables,
        ["connection_log", "devices", "profiles", "settings"]
    );

    // Em tabelas STRICT a chave primária de texto já é NOT NULL.
    assert_eq!(
        columns(pool, "settings").await,
        [
            col("key", "TEXT", true, true),
            col("value", "TEXT", true, false),
            col("updated_at", "INTEGER", true, false),
        ]
    );
    assert_eq!(
        columns(pool, "devices").await,
        [
            col("id", "TEXT", true, true),
            col("name", "TEXT", true, false),
            col("model", "TEXT", false, false),
            col("os_version", "TEXT", false, false),
            col("token_hash", "BLOB", true, false),
            col("paired_at", "INTEGER", true, false),
            col("last_seen_at", "INTEGER", false, false),
            col("last_ip", "TEXT", false, false),
            col("revoked", "INTEGER", true, false),
            col("preferred_slot", "INTEGER", false, false),
            col("default_profile_id", "TEXT", false, false),
            col("color", "TEXT", false, false),
        ]
    );
    let profile_columns: Vec<String> = columns(pool, "profiles")
        .await
        .into_iter()
        .map(|c| c.0)
        .collect();
    assert_eq!(
        profile_columns,
        [
            "id",
            "name",
            "description",
            "icon",
            "is_builtin",
            "schema_version",
            "config",
            "created_at",
            "updated_at"
        ]
    );
    let log_columns: Vec<String> = columns(pool, "connection_log")
        .await
        .into_iter()
        .map(|c| c.0)
        .collect();
    assert_eq!(
        log_columns,
        ["id", "device_id", "event", "slot", "detail", "at"]
    );

    let indexes: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'index' AND name LIKE 'idx\\_%' ESCAPE '\\' \
         ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(
        indexes,
        [
            "idx_connection_log_at",
            "idx_connection_log_device",
            "idx_devices_default_profile"
        ]
    );
}

#[tokio::test]
async fn tables_are_strict_and_reject_values_of_the_wrong_type() {
    let db = Database::in_memory().await.unwrap();
    let error =
        sqlx::query("INSERT INTO settings (key, value, updated_at) VALUES ('k', 'v', 'ontem')")
            .execute(db.pool())
            .await;
    assert!(
        error.is_err(),
        "STRICT deve recusar texto em coluna INTEGER"
    );
}

#[tokio::test]
async fn slot_and_flag_checks_are_enforced() {
    let db = Database::in_memory().await.unwrap();
    let pool = db.pool();
    insert_device(pool, "d1").await;
    for slot in [0, 3] {
        sqlx::query("UPDATE devices SET preferred_slot = ? WHERE id = 'd1'")
            .bind(slot)
            .execute(pool)
            .await
            .unwrap();
    }
    for slot in [-1, 4] {
        assert!(
            sqlx::query("UPDATE devices SET preferred_slot = ? WHERE id = 'd1'")
                .bind(slot)
                .execute(pool)
                .await
                .is_err(),
            "slot {slot} deveria ser recusado"
        );
    }
    assert!(
        sqlx::query("UPDATE devices SET revoked = 2 WHERE id = 'd1'")
            .execute(pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE profiles SET is_builtin = 5")
            .execute(pool)
            .await
            .is_err()
    );
    // NULL é "sem preferência".
    sqlx::query("UPDATE devices SET preferred_slot = NULL")
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn foreign_keys_cascade_and_set_null_as_documented() {
    let db = Database::in_memory().await.unwrap();
    let pool = db.pool();

    // O dispositivo precisa de um perfil que exista.
    assert!(
        sqlx::query(
            "INSERT INTO devices (id, name, token_hash, paired_at, default_profile_id) \
             VALUES ('d1', 'Pixel', x'00', 1, 'nao-existe')"
        )
        .execute(pool)
        .await
        .is_err()
    );
    sqlx::query(
        "INSERT INTO devices (id, name, token_hash, paired_at, default_profile_id) \
         VALUES ('d1', 'Pixel', x'00', 1, ?)",
    )
    .bind(BUILTIN_PROFILE_ID)
    .execute(pool)
    .await
    .unwrap();
    // Um perfil do usuário, apagado: o dispositivo volta a "sem perfil" (SET NULL).
    sqlx::query(
        "INSERT INTO profiles (id, name, schema_version, config, created_at, updated_at) \
         VALUES ('meu', 'Meu', 1, '{}', 1, 1)",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("UPDATE devices SET default_profile_id = 'meu' WHERE id = 'd1'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM profiles WHERE id = 'meu'")
        .execute(pool)
        .await
        .unwrap();
    let profile: Option<String> =
        sqlx::query_scalar("SELECT default_profile_id FROM devices WHERE id = 'd1'")
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(profile, None);

    // O histórico exige um dispositivo e some junto com ele (CASCADE).
    assert!(
        sqlx::query(
            "INSERT INTO connection_log (device_id, event, at) VALUES ('fantasma', 'paired', 1)"
        )
        .execute(pool)
        .await
        .is_err()
    );
    sqlx::query(
        "INSERT INTO connection_log (device_id, event, slot, at) VALUES ('d1', 'connected', 0, 1)",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("DELETE FROM devices WHERE id = 'd1'")
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(count(pool, "SELECT count(*) FROM connection_log").await, 0);
}

#[tokio::test]
async fn seeds_create_the_default_settings_and_the_builtin_profile() {
    let db = Database::in_memory().await.unwrap();
    let pool = db.pool();

    for (key, value) in DEFAULT_SETTINGS {
        assert_eq!(setting(pool, key).await.as_deref(), Some(value), "{key}");
        // Todo valor é JSON válido.
        assert!(
            serde_json::from_str::<serde_json::Value>(value).is_ok(),
            "{key}"
        );
    }
    assert_eq!(
        setting(pool, "language").await.as_deref(),
        Some("\"pt-BR\"")
    );
    assert_eq!(
        setting(pool, "accent").await.as_deref(),
        Some("\"#0A84FF\"")
    );
    assert_eq!(
        setting(pool, "ports").await.as_deref(),
        Some(r#"{"control":47800,"input":47801,"discovery":47802}"#)
    );

    let row =
        sqlx::query("SELECT name, is_builtin, schema_version, config FROM profiles WHERE id = ?")
            .bind(BUILTIN_PROFILE_ID)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(row.get::<String, _>("name"), BUILTIN_PROFILE_NAME);
    assert_eq!(row.get::<i64, _>("is_builtin"), 1);
    // O JSON gravado é um perfil válido para o `padgaming-mapping`, e compila.
    let profile = Profile::from_json(&row.get::<String, _>("config")).unwrap();
    assert_eq!(
        row.get::<i64, _>("schema_version"),
        i64::from(profile.schema_version)
    );
    assert_eq!(profile, Profile::standard());
    CompiledProfile::compile(&profile).unwrap();
}

#[tokio::test]
async fn seeds_are_idempotent_and_never_overwrite_user_choices() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).await.unwrap();
    let pool = db.pool();
    let settings_before = count(pool, "SELECT count(*) FROM settings").await;

    // O usuário muda uma configuração e cria um perfil próprio.
    sqlx::query("UPDATE settings SET value = '\"en\"', updated_at = 42 WHERE key = 'language'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO profiles (id, name, schema_version, config, created_at, updated_at) \
         VALUES ('meu', 'Meu', 1, '{\"x\":1}', 1, 1)",
    )
    .execute(pool)
    .await
    .unwrap();
    db.close().await;

    for _ in 0..3 {
        Database::open(dir.path()).await.unwrap().close().await;
    }
    let db = Database::open(dir.path()).await.unwrap();
    let pool = db.pool();
    assert_eq!(
        count(pool, "SELECT count(*) FROM settings").await,
        settings_before
    );
    assert_eq!(setting(pool, "language").await.as_deref(), Some("\"en\""));
    assert_eq!(count(pool, "SELECT count(*) FROM profiles").await, 2);
    assert_eq!(
        count(pool, "SELECT count(*) FROM profiles WHERE is_builtin = 1").await,
        1
    );
    let mine: String = sqlx::query_scalar("SELECT config FROM profiles WHERE id = 'meu'")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(mine, "{\"x\":1}");
}

#[tokio::test]
async fn the_builtin_profile_is_restored_and_only_touched_when_it_changed() {
    let db = Database::in_memory().await.unwrap();
    let pool = db.pool();
    let updated = |pool| async move {
        sqlx::query_scalar::<_, i64>("SELECT updated_at FROM profiles WHERE id = ?")
            .bind(BUILTIN_PROFILE_ID)
            .fetch_one(pool)
            .await
            .unwrap()
    };
    let first = updated(pool).await;

    // Reaplicar sem mudança não mexe no `updated_at`.
    seed::apply(pool, first + 10_000).await.unwrap();
    assert_eq!(updated(pool).await, first);

    // Alguém adulterou o embutido: volta ao que o app traz.
    sqlx::query("UPDATE profiles SET config = '{}', name = 'Hackeado' WHERE id = ?")
        .bind(BUILTIN_PROFILE_ID)
        .execute(pool)
        .await
        .unwrap();
    seed::apply(pool, first + 20_000).await.unwrap();
    let name: String = sqlx::query_scalar("SELECT name FROM profiles WHERE id = ?")
        .bind(BUILTIN_PROFILE_ID)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(name, BUILTIN_PROFILE_NAME);
    assert_eq!(updated(pool).await, first + 20_000);

    // Apagado, é recriado.
    sqlx::query("DELETE FROM profiles WHERE id = ?")
        .bind(BUILTIN_PROFILE_ID)
        .execute(pool)
        .await
        .unwrap();
    seed::apply(pool, first + 30_000).await.unwrap();
    assert_eq!(
        count(pool, "SELECT count(*) FROM profiles WHERE is_builtin = 1").await,
        1
    );
}

async fn fill_log(pool: &SqlitePool, rows: i64, at: i64) {
    sqlx::query(
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < ?) \
         INSERT INTO connection_log (event, at) SELECT 'connected', ? FROM n",
    )
    .bind(rows)
    .bind(at)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn the_log_drops_entries_older_than_30_days() {
    let db = Database::in_memory().await.unwrap();
    let pool = db.pool();
    fill_log(pool, 5, NOW - LOG_MAX_AGE_MS - 1).await; // velhas
    fill_log(pool, 3, NOW - LOG_MAX_AGE_MS).await; // no limite: ficam
    fill_log(pool, 2, NOW).await;

    let pruned = seed::prune_connection_log(pool, NOW).await.unwrap();
    assert_eq!(pruned, 5);
    assert_eq!(count(pool, "SELECT count(*) FROM connection_log").await, 5);
    // Idempotente.
    assert_eq!(seed::prune_connection_log(pool, NOW).await.unwrap(), 0);
}

#[tokio::test]
async fn the_log_keeps_only_the_newest_ten_thousand_rows() {
    let db = Database::in_memory().await.unwrap();
    let pool = db.pool();
    fill_log(pool, LOG_MAX_ROWS + 250, NOW).await;
    assert_eq!(
        count(pool, "SELECT count(*) FROM connection_log").await,
        LOG_MAX_ROWS + 250
    );

    let pruned = seed::prune_connection_log(pool, NOW).await.unwrap();
    assert_eq!(pruned, 250);
    assert_eq!(
        count(pool, "SELECT count(*) FROM connection_log").await,
        LOG_MAX_ROWS
    );
    // Ficaram as mais recentes (ids maiores), não as mais antigas.
    assert_eq!(count(pool, "SELECT min(id) FROM connection_log").await, 251);
    // Abaixo do teto não apaga nada.
    assert_eq!(seed::prune_connection_log(pool, NOW).await.unwrap(), 0);
}

#[tokio::test]
async fn the_boot_applies_the_log_retention() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).await.unwrap();
    fill_log(db.pool(), 4, 1_000).await; // de 1970
    fill_log(db.pool(), 1, seed::now_ms()).await;
    db.close().await;

    let db = Database::open(dir.path()).await.unwrap();
    assert_eq!(
        count(db.pool(), "SELECT count(*) FROM connection_log").await,
        1
    );
}

#[tokio::test]
async fn migration_1_applies_on_an_empty_database_and_a_later_one_on_top_of_it() {
    assert_eq!(MIGRATOR.iter().map(|m| m.version).collect::<Vec<_>>(), [1]);

    // Um banco no schema v1 com dados...
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).await.unwrap();
    insert_device(db.pool(), "d1").await;
    db.close().await;

    // ... recebe uma migration futura (simulada) por cima do v1 e continua íntegro.
    let mut migrations: Vec<Migration> = MIGRATOR.iter().cloned().collect();
    migrations.push(Migration::new(
        2,
        Cow::Borrowed("device_notes"),
        MigrationType::Simple,
        "ALTER TABLE devices ADD COLUMN notes TEXT;".into_sql_str(),
        false,
    ));
    let next = Migrator::with_migrations(migrations);
    let db = Database::open_with(dir.path(), &next).await.unwrap();
    assert_eq!(count(db.pool(), "SELECT count(*) FROM devices").await, 1);
    assert_eq!(
        count(
            db.pool(),
            "SELECT count(*) FROM pragma_table_info('devices') WHERE name = 'notes'"
        )
        .await,
        1
    );
    // O backup do v1 foi feito antes.
    assert!(dir.path().join(format!("{DB_FILE}.bak-1")).is_file());
}
