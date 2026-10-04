//! Dados iniciais (seeds) e manutenção no boot. Tudo aqui é idempotente: rodar em todo boot
//! não muda nada que já esteja certo.

use std::time::{SystemTime, UNIX_EPOCH};

use padgaming_mapping::{Profile, SCHEMA_VERSION};
use sqlx::{SqliteConnection, SqlitePool};

/// `id` do perfil "Padrão" embutido. Fixo, para o app achá-lo e para o seed ser idempotente.
pub const BUILTIN_PROFILE_ID: &str = "00000000-0000-4000-8000-000000000001";
/// Nome do perfil embutido.
pub const BUILTIN_PROFILE_NAME: &str = "Padrão";

/// Configurações iniciais: chave e valor em JSON. Só são gravadas se a chave não existir: o que
/// o usuário mudou nunca é sobrescrito.
pub const DEFAULT_SETTINGS: [(&str, &str); 15] = [
    ("language", r#""pt-BR""#),
    ("theme", r#""dark""#),
    ("accent", r##""#0A84FF""##),
    (
        "ports",
        r#"{"control":47800,"input":47801,"discovery":47802}"#,
    ),
    ("autostart", "false"),
    ("start_minimized", "false"),
    ("onboarding_completed", "false"),
    ("require_approval", "false"),
    ("server_name", r#""""#),
    ("close_to_tray", "true"),
    ("default_profile_id", "null"),
    ("send_rate_hz", "250"),
    ("reservation_secs", "30"),
    ("transparency", "true"),
    ("reduce_motion", "false"),
];

/// Quanto tempo o `connection_log` guarda (30 dias).
pub const LOG_MAX_AGE_MS: i64 = 30 * 24 * 60 * 60 * 1000;
/// Quantas linhas o `connection_log` guarda, no máximo.
pub const LOG_MAX_ROWS: i64 = 10_000;

/// Epoch ms de agora (UTC).
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// Grava as configurações que faltam e cria/atualiza o perfil "Padrão".
///
/// O perfil embutido é reescrito a cada boot com a versão que este app traz (o usuário edita
/// cópias, não o embutido; "Restaurar padrão" da T-604 usa o mesmo caminho). Um perfil do usuário
/// nunca é tocado.
pub async fn apply(pool: &SqlitePool, now: i64) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    apply_settings_in(&mut tx, now).await?;
    apply_builtin_profile_in(&mut tx, now).await?;
    tx.commit().await
}

/// Só o perfil "Padrão" (recria se sumiu, repõe se foi adulterado).
pub async fn apply_builtin_profile(pool: &SqlitePool, now: i64) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    apply_builtin_profile_in(&mut tx, now).await?;
    tx.commit().await
}

async fn apply_settings_in(tx: &mut SqliteConnection, now: i64) -> Result<(), sqlx::Error> {
    for (key, value) in DEFAULT_SETTINGS {
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)              ON CONFLICT (key) DO NOTHING",
        )
        .bind(key)
        .bind(value)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

async fn apply_builtin_profile_in(tx: &mut SqliteConnection, now: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO profiles              (id, name, description, icon, is_builtin, schema_version, config, created_at, updated_at)          VALUES (?, ?, ?, ?, 1, ?, ?, ?, ?)          ON CONFLICT (id) DO UPDATE SET              name = excluded.name, description = excluded.description, icon = excluded.icon,              is_builtin = 1, schema_version = excluded.schema_version, config = excluded.config,              updated_at = CASE WHEN profiles.config = excluded.config                                THEN profiles.updated_at ELSE excluded.updated_at END",
    )
    .bind(BUILTIN_PROFILE_ID)
    .bind(BUILTIN_PROFILE_NAME)
    .bind("Sticks com zona morta de 8%, sem gyro")
    .bind("gamecontroller")
    .bind(i64::from(SCHEMA_VERSION))
    .bind(Profile::standard().to_json())
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Aplica a retenção do `connection_log`: apaga o que passou de 30 dias e, do que sobrar, tudo
/// além das 10 mil linhas mais recentes. Devolve quantas linhas apagou.
pub async fn prune_connection_log(pool: &SqlitePool, now: i64) -> Result<u64, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let old = sqlx::query("DELETE FROM connection_log WHERE at < ?")
        .bind(now - LOG_MAX_AGE_MS)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    let excess = sqlx::query(
        "DELETE FROM connection_log WHERE id <= \
             (SELECT id FROM connection_log ORDER BY id DESC LIMIT 1 OFFSET ?)",
    )
    .bind(LOG_MAX_ROWS)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    tx.commit().await?;
    Ok(old + excess)
}
