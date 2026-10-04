-- Schema v1 do VynPad (T-402).
--
-- Timestamps em epoch ms (UTC). Booleanos são INTEGER 0/1. As tabelas são STRICT: o SQLite
-- recusa um tipo errado em vez de converter em silêncio. NUNCA edite este arquivo depois de
-- publicado (o sqlx confere o checksum): mudanças vão em uma migration nova.

-- Configurações chave/valor; `value` é JSON.
CREATE TABLE settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;

-- Perfis de mapeamento. `config` é o JSON do `vynpad-mapping`, validado por serde e por
-- `schema_version` (evolui junto com o crate, sem dezenas de colunas).
CREATE TABLE profiles (
    id             TEXT PRIMARY KEY,                       -- uuid
    name           TEXT NOT NULL,
    description    TEXT,
    icon           TEXT,                                   -- nome do ícone/emoji do perfil
    is_builtin     INTEGER NOT NULL DEFAULT 0 CHECK (is_builtin IN (0, 1)),
    schema_version INTEGER NOT NULL,
    config         TEXT NOT NULL,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL
) STRICT;

-- Dispositivos pareados. O token nunca é guardado, só o SHA-256 dele.
CREATE TABLE devices (
    id                 TEXT PRIMARY KEY,                   -- device_id enviado pelo client
    name               TEXT NOT NULL,                      -- editável pelo usuário
    model              TEXT,
    os_version         TEXT,
    token_hash         BLOB NOT NULL,
    paired_at          INTEGER NOT NULL,
    last_seen_at       INTEGER,
    last_ip            TEXT,
    revoked            INTEGER NOT NULL DEFAULT 0 CHECK (revoked IN (0, 1)),
    preferred_slot     INTEGER CHECK (preferred_slot BETWEEN 0 AND 3),
    default_profile_id TEXT REFERENCES profiles (id) ON DELETE SET NULL,
    color              TEXT                                -- sobrescreve a cor do slot na UI
) STRICT;

-- Histórico de conexões (retenção: 30 dias / 10 mil linhas, aplicada no boot).
CREATE TABLE connection_log (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT REFERENCES devices (id) ON DELETE CASCADE,
    event     TEXT NOT NULL,        -- connected | disconnected | rejected | paired | revoked
    slot      INTEGER,
    detail    TEXT,
    at        INTEGER NOT NULL
) STRICT;

CREATE INDEX idx_connection_log_at ON connection_log (at);
-- Apagar um dispositivo (ON DELETE CASCADE) e listar o histórico dele.
CREATE INDEX idx_connection_log_device ON connection_log (device_id);
-- "Perfil padrão do dispositivo" (ON DELETE SET NULL) percorre a tabela de dispositivos.
CREATE INDEX idx_devices_default_profile ON devices (default_profile_id);
