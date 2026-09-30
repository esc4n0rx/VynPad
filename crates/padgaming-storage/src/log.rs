//! Histórico de conexões (`connection_log`).

use sqlx::SqlitePool;

use crate::error::DataError;

/// Máximo de linhas por consulta.
pub const MAX_PAGE: u32 = 500;

/// O que aconteceu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogEvent {
    /// Uma sessão começou.
    Connected,
    /// Uma sessão terminou.
    Disconnected,
    /// Uma conexão foi recusada.
    Rejected,
    /// Primeiro pareamento concluído.
    Paired,
    /// O dispositivo foi revogado.
    Revoked,
}

impl LogEvent {
    /// O texto gravado na coluna `event`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::Disconnected => "disconnected",
            Self::Rejected => "rejected",
            Self::Paired => "paired",
            Self::Revoked => "revoked",
        }
    }
}

/// Uma linha do histórico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    /// Identificador (cresce com o tempo).
    pub id: i64,
    /// Dispositivo (`None` para uma conexão recusada de alguém desconhecido).
    pub device_id: Option<String>,
    /// Evento (`connected`, `disconnected`, `rejected`, `paired`, `revoked`).
    pub event: String,
    /// Slot envolvido.
    pub slot: Option<u8>,
    /// Detalhe livre (motivo da recusa, IP...).
    pub detail: Option<String>,
    /// Quando, em epoch ms.
    pub at: i64,
}

/// Filtro e paginação da consulta.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogQuery {
    /// Só os eventos deste dispositivo.
    pub device_id: Option<String>,
    /// Só os eventos anteriores a este `id` (para a próxima página).
    pub before_id: Option<i64>,
    /// Quantas linhas (1–[`MAX_PAGE`]); 0 vale o padrão de 100.
    pub limit: u32,
}

/// Leitura e gravação do histórico.
#[derive(Debug, Clone)]
pub struct ConnectionLog {
    pool: SqlitePool,
}

impl ConnectionLog {
    /// Repositório sobre `pool`.
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Registra um evento.
    pub async fn append(
        &self,
        device_id: Option<&str>,
        event: LogEvent,
        slot: Option<u8>,
        detail: Option<&str>,
        at: i64,
    ) -> Result<(), DataError> {
        let event = event.as_str();
        let slot = slot.map(i64::from);
        sqlx::query!(
            "INSERT INTO connection_log (device_id, event, slot, detail, at) VALUES (?, ?, ?, ?, ?)",
            device_id,
            event,
            slot,
            detail,
            at
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// As linhas mais recentes primeiro.
    pub async fn list(&self, query: &LogQuery) -> Result<Vec<LogEntry>, DataError> {
        let limit = i64::from(match query.limit {
            0 => 100,
            n => n.min(MAX_PAGE),
        });
        // `IS NULL OR` deixa os dois filtros opcionais numa única query verificada.
        let rows = sqlx::query!(
            "SELECT id, device_id, event, slot, detail, at FROM connection_log \
             WHERE (?1 IS NULL OR device_id = ?1) AND (?2 IS NULL OR id < ?2) \
             ORDER BY id DESC LIMIT ?3",
            query.device_id,
            query.before_id,
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| LogEntry {
                id: r.id,
                device_id: r.device_id,
                event: r.event,
                slot: r.slot.and_then(|s| u8::try_from(s).ok()),
                detail: r.detail,
                at: r.at,
            })
            .collect())
    }
}
