# T-402 — Schema v1

| Campo | Valor |
|---|---|
| Fase | F4 — Persistência |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-401 |
| Status | ⬜ A fazer |

## Objetivo
Modelar tudo o que precisa sobreviver a reinícios.

## Escopo

```sql
CREATE TABLE settings (
  key         TEXT PRIMARY KEY,
  value       TEXT NOT NULL,              -- JSON
  updated_at  INTEGER NOT NULL
);

CREATE TABLE profiles (
  id             TEXT PRIMARY KEY,        -- uuid
  name           TEXT NOT NULL,
  description    TEXT,
  icon           TEXT,                    -- nome do ícone/emoji do perfil
  is_builtin     INTEGER NOT NULL DEFAULT 0,
  schema_version INTEGER NOT NULL,
  config         TEXT NOT NULL,           -- JSON validado pelo padgaming-mapping
  created_at     INTEGER NOT NULL,
  updated_at     INTEGER NOT NULL
);

CREATE TABLE devices (
  id                 TEXT PRIMARY KEY,    -- device_id enviado pelo client
  name               TEXT NOT NULL,       -- editável pelo usuário
  model              TEXT,
  os_version         TEXT,
  token_hash         BLOB NOT NULL,
  paired_at          INTEGER NOT NULL,
  last_seen_at       INTEGER,
  last_ip            TEXT,
  revoked            INTEGER NOT NULL DEFAULT 0,
  preferred_slot     INTEGER CHECK (preferred_slot BETWEEN 0 AND 3),
  default_profile_id TEXT REFERENCES profiles(id) ON DELETE SET NULL,
  color              TEXT                 -- sobrescreve a cor do slot na UI (opcional)
);

CREATE TABLE connection_log (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  device_id  TEXT REFERENCES devices(id) ON DELETE CASCADE,
  event      TEXT NOT NULL,              -- connected | disconnected | rejected | paired | revoked
  slot       INTEGER,
  detail     TEXT,
  at         INTEGER NOT NULL
);
CREATE INDEX idx_connection_log_at ON connection_log(at);
```

- Seed: perfil "Padrão" (builtin) + configurações padrão (`language=pt-BR`, `theme=dark`, `accent=#0A84FF`, `ports`, `autostart=false`, `start_minimized=false`, `onboarding_completed=false`, `require_approval=false`).
- Retenção do `connection_log`: 30 dias / 10 mil linhas (limpeza no boot).

## Detalhes técnicos
- Timestamps em epoch ms (UTC).
- O `config` do perfil é JSON (evolui junto com o crate de mapping, validado por serde + `schema_version`) em vez de dezenas de colunas.

## Critérios de aceite
- [ ] Migration aplicada num banco vazio e sobre a versão anterior (teste).
- [ ] Seeds idempotentes.

## Fora de escopo
- Acesso via código (T-403).
