//! Dispositivos pareados (`devices`): o que o pareamento grava e o que a UI edita.

use sqlx::SqlitePool;

use crate::error::DataError;
use crate::log::{ConnectionLog, LogEvent};

/// Tamanho máximo do nome exibido.
pub const MAX_NAME_LEN: usize = 64;

/// Um dispositivo pareado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// `device_id` enviado pelo client.
    pub id: String,
    /// Nome exibido (editável).
    pub name: String,
    /// Modelo.
    pub model: Option<String>,
    /// Versão do SO.
    pub os_version: Option<String>,
    /// SHA-256 do `device_token`. O token em si nunca é guardado.
    pub token_hash: [u8; 32],
    /// Quando pareou (epoch ms).
    pub paired_at: i64,
    /// Última conexão (epoch ms).
    pub last_seen_at: Option<i64>,
    /// IP da última conexão.
    pub last_ip: Option<String>,
    /// Revogado pelo usuário.
    pub revoked: bool,
    /// Slot preferido (0–3).
    pub preferred_slot: Option<u8>,
    /// Perfil padrão do dispositivo.
    pub default_profile_id: Option<String>,
    /// Cor que sobrescreve a do slot (`#RRGGBB`).
    pub color: Option<String>,
}

/// O que o pareamento sabe sobre o dispositivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPairing {
    /// `device_id`.
    pub id: String,
    /// Nome enviado no `hello`.
    pub name: String,
    /// Modelo.
    pub model: Option<String>,
    /// Versão do SO.
    pub os_version: Option<String>,
    /// SHA-256 do `device_token` recém-emitido.
    pub token_hash: [u8; 32],
    /// Agora (epoch ms).
    pub now: i64,
}

/// Preferências do dispositivo. `None` limpa o campo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceDefaults {
    /// Slot preferido (0–3).
    pub preferred_slot: Option<u8>,
    /// Perfil padrão (precisa existir).
    pub default_profile_id: Option<String>,
    /// Cor `#RRGGBB`.
    pub color: Option<String>,
}

struct Row {
    id: String,
    name: String,
    model: Option<String>,
    os_version: Option<String>,
    token_hash: Vec<u8>,
    paired_at: i64,
    last_seen_at: Option<i64>,
    last_ip: Option<String>,
    revoked: bool,
    preferred_slot: Option<i64>,
    default_profile_id: Option<String>,
    color: Option<String>,
}

impl TryFrom<Row> for Device {
    type Error = DataError;

    fn try_from(row: Row) -> Result<Self, DataError> {
        let token_hash = <[u8; 32]>::try_from(row.token_hash.as_slice()).map_err(|_| {
            DataError::Invalid("o hash do token do dispositivo está corrompido".into())
        })?;
        Ok(Self {
            id: row.id,
            name: row.name,
            model: row.model,
            os_version: row.os_version,
            token_hash,
            paired_at: row.paired_at,
            last_seen_at: row.last_seen_at,
            last_ip: row.last_ip,
            revoked: row.revoked,
            preferred_slot: row.preferred_slot.and_then(|s| u8::try_from(s).ok()),
            default_profile_id: row.default_profile_id,
            color: row.color,
        })
    }
}

/// Valida um nome de dispositivo (1–64 caracteres, sem só espaços).
pub fn validate_name(name: &str) -> Result<String, DataError> {
    let name = name.trim();
    let len = name.chars().count();
    if len == 0 || len > MAX_NAME_LEN {
        return Err(DataError::Invalid(format!(
            "o nome precisa ter de 1 a {MAX_NAME_LEN} caracteres"
        )));
    }
    Ok(name.to_owned())
}

fn validate_color(color: &str) -> Result<(), DataError> {
    let hex = color.strip_prefix('#').unwrap_or("");
    if hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(DataError::Invalid(
            "a cor precisa estar no formato #RRGGBB".into(),
        ))
    }
}

/// Leitura e gravação da tabela `devices`.
#[derive(Debug, Clone)]
pub struct DeviceRepo {
    pool: SqlitePool,
    log: ConnectionLog,
}

impl DeviceRepo {
    /// Repositório sobre `pool`.
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            log: ConnectionLog::new(pool.clone()),
            pool,
        }
    }

    /// Busca pelo `device_id`.
    pub async fn find(&self, id: &str) -> Result<Option<Device>, DataError> {
        sqlx::query_as!(
            Row,
            r#"SELECT id, name, model, os_version, token_hash, paired_at, last_seen_at, last_ip,
                      revoked AS "revoked: bool", preferred_slot, default_profile_id, color
               FROM devices WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?
        .map(Device::try_from)
        .transpose()
    }

    /// Todos os dispositivos: os vistos há menos tempo primeiro, depois os nunca vistos.
    pub async fn list(&self) -> Result<Vec<Device>, DataError> {
        sqlx::query_as!(
            Row,
            r#"SELECT id, name, model, os_version, token_hash, paired_at, last_seen_at, last_ip,
                      revoked AS "revoked: bool", preferred_slot, default_profile_id, color
               FROM devices
               ORDER BY COALESCE(last_seen_at, paired_at) DESC, id"#
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Device::try_from)
        .collect()
    }

    /// Grava um pareamento. Parear de novo um `device_id` já conhecido troca o token e remove a
    /// revogação (o usuário autorizou de novo com QR/PIN), mas mantém o slot, o perfil e a cor
    /// que ele já tinha. Registra `paired` no histórico.
    pub async fn upsert_pairing(&self, pairing: &NewPairing) -> Result<(), DataError> {
        let name = validate_name(&pairing.name)?;
        let token_hash = pairing.token_hash.as_slice();
        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            "INSERT INTO devices (id, name, model, os_version, token_hash, paired_at, revoked) \
             VALUES (?, ?, ?, ?, ?, ?, 0) \
             ON CONFLICT (id) DO UPDATE SET \
                 name = excluded.name, model = excluded.model, os_version = excluded.os_version, \
                 token_hash = excluded.token_hash, paired_at = excluded.paired_at, revoked = 0",
            pairing.id,
            name,
            pairing.model,
            pairing.os_version,
            token_hash,
            pairing.now
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "INSERT INTO connection_log (device_id, event, at) VALUES (?, 'paired', ?)",
            pairing.id,
            pairing.now
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Renomeia. Devolve o nome gravado (já sem espaços nas pontas).
    pub async fn rename(&self, id: &str, name: &str) -> Result<String, DataError> {
        let name = validate_name(name)?;
        let done = sqlx::query!("UPDATE devices SET name = ? WHERE id = ?", name, id)
            .execute(&self.pool)
            .await?;
        if done.rows_affected() == 0 {
            return Err(DataError::NotFound("dispositivo"));
        }
        Ok(name)
    }

    /// Revoga o dispositivo (o `device_token` deixa de valer) e registra no histórico.
    pub async fn revoke(&self, id: &str, now: i64) -> Result<(), DataError> {
        let done = sqlx::query!("UPDATE devices SET revoked = 1 WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        if done.rows_affected() == 0 {
            return Err(DataError::NotFound("dispositivo"));
        }
        self.log
            .append(Some(id), LogEvent::Revoked, None, None, now)
            .await
    }

    /// Esquece o dispositivo: apaga o registro e o histórico dele (CASCADE). Ele precisa
    /// parear de novo.
    pub async fn forget(&self, id: &str) -> Result<(), DataError> {
        let done = sqlx::query!("DELETE FROM devices WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        if done.rows_affected() == 0 {
            return Err(DataError::NotFound("dispositivo"));
        }
        Ok(())
    }

    /// Define slot preferido, perfil padrão e cor.
    pub async fn set_defaults(&self, id: &str, defaults: &DeviceDefaults) -> Result<(), DataError> {
        if let Some(slot) = defaults.preferred_slot
            && slot > 3
        {
            return Err(DataError::Invalid("o slot precisa ser de 0 a 3".into()));
        }
        if let Some(color) = &defaults.color {
            validate_color(color)?;
        }
        let slot = defaults.preferred_slot.map(i64::from);
        let done = sqlx::query!(
            "UPDATE devices SET preferred_slot = ?, default_profile_id = ?, color = ? WHERE id = ?",
            slot,
            defaults.default_profile_id,
            defaults.color,
            id
        )
        .execute(&self.pool)
        .await;
        match done {
            Ok(done) if done.rows_affected() == 0 => Err(DataError::NotFound("dispositivo")),
            Ok(_) => Ok(()),
            // A chave estrangeira recusou: o perfil não existe.
            Err(sqlx::Error::Database(db)) if db.is_foreign_key_violation() => {
                Err(DataError::NotFound("perfil"))
            }
            Err(other) => Err(other.into()),
        }
    }

    /// Anota quando e de onde o dispositivo foi visto por último.
    pub async fn touch_seen(&self, id: &str, ip: Option<&str>, now: i64) -> Result<(), DataError> {
        sqlx::query!(
            "UPDATE devices SET last_seen_at = ?, last_ip = COALESCE(?, last_ip) WHERE id = ?",
            now,
            ip,
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
