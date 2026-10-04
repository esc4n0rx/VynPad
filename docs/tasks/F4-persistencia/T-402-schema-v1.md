# T-402 — Schema v1

| Campo | Valor |
|---|---|
| Fase | F4 — Persistência |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-401 |
| Status | ✅ Concluída |

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
  config         TEXT NOT NULL,           -- JSON validado pelo vynpad-mapping
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
- [x] Migration aplicada num banco vazio e sobre a versão anterior (teste).
- [x] Seeds idempotentes.

## Fora de escopo
- Acesso via código (T-403).

## Situação

Pronto em `vynpad-storage`:
- `migrations/0001_schema.sql` com as quatro tabelas do escopo. Diferenças em relação ao SQL do escopo, todas para dar mais garantia sem mudar o modelo: tabelas `STRICT` (o SQLite recusa tipo errado em vez de converter), `CHECK (revoked IN (0, 1))` e `CHECK (is_builtin IN (0, 1))`, e dois índices a mais (`idx_connection_log_device` para o `ON DELETE CASCADE` e o histórico por dispositivo, e `idx_devices_default_profile` para o `ON DELETE SET NULL`). Em tabela `STRICT` a chave primária de texto já é `NOT NULL`. Não pus `CHECK` em `connection_log.event` para novos eventos não exigirem recriar a tabela.
- Seeds em `seed::apply`, rodados em todo `Database::open`/`in_memory` (não em `open_with`, que é para migrations de teste): as 8 configurações padrão só entram se a chave não existir (nunca sobrescrevem o que o usuário mudou), e o perfil "Padrão" (`BUILTIN_PROFILE_ID`, `is_builtin = 1`, JSON do `Profile::standard()` do `vynpad-mapping`) é recriado se sumir e reescrito se for adulterado, sem mexer no `updated_at` quando nada mudou. Perfis do usuário nunca são tocados. Os seeds ficam no Rust, e não no SQL, para o JSON acompanhar o `vynpad-mapping`.
- Retenção em `seed::prune_connection_log`, também a cada boot: apaga mais de 30 dias e, do resto, tudo além das 10 mil linhas mais recentes.
- Migration sobre a versão anterior: só existe a v1, então o teste aplica a v1 num banco vazio e uma migration futura simulada por cima (dados preservados, backup `.bak-1` criado).

Testes (11, em `tests/schema.rs`): tabelas e colunas exatas, `STRICT`, `CHECK`s de slot e flags, chaves estrangeiras (CASCADE e SET NULL), seeds, idempotência com escolhas do usuário, restauração do perfil embutido, e retenção por idade, por quantidade e no boot.
