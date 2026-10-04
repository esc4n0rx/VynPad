# T-403 — Repositórios, serviços e commands

| Campo | Valor |
|---|---|
| Fase | F4 — Persistência |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-402, T-506 |
| Status | ✅ Concluída |

## Objetivo
Expor os dados de forma tipada para o core e para a UI.

## Escopo
- Repositórios: `SettingsRepo`, `ProfileRepo`, `DeviceRepo`, `ConnectionLogRepo`.
- `SettingsService` com struct tipada `AppSettings` (serde, com defaults) + evento `settings:changed`.
- `ProfileService`: CRUD, duplicar, importar/exportar JSON (arquivo `.padprofile`), validar + compilar → notificar o `SlotManager` (hot swap).
- `DeviceService`: implementa o `DeviceStore` da T-103; renomear, revogar, esquecer, definir slot/perfil padrão.
- Commands Tauri (tipados via T-506):
  `get_settings`, `update_settings`, `list_profiles`, `get_profile`, `save_profile`, `delete_profile`, `duplicate_profile`, `import_profile`, `export_profile`, `list_devices`, `rename_device`, `revoke_device`, `forget_device`, `set_device_defaults`, `get_connection_log`.

## Critérios de aceite
- [x] Testes de integração dos repositórios com o banco em memória.
- [x] Editar um perfil na UI altera o comportamento do slot sem reconectar. (`ProfileService::save` → `SlotRegistry` (host real) troca o `ProfileCell` do slot; teste de ponta a ponta em `vynpad-server/tests/core.rs`.)
- [x] Não é possível apagar o perfil builtin nem um perfil em uso sem confirmação.

## Fora de escopo
- Telas (F6).

## Situação

Camada Rust pronta em `vynpad-storage` (o que não depende do Tauri):
- `SettingsService`: `AppSettings` tipada (idioma, tema, cor, portas, autostart, iniciar minimizado, onboarding, confirmação de novos dispositivos), validada (cor `#RRGGBB`, portas ≥ 1024, diferentes entre si e ≠ 5353), cache em memória, grava só se mudou e avisa por `subscribe()` (o `settings:changed`). Um valor corrompido no banco volta ao padrão só naquele campo.
- `ProfileService`: criar, `save` (aplica na hora via `ProfileHost::apply`, sem reconectar), duplicar (inclusive o embutido), apagar (o embutido nunca; em uso por dispositivo ou controle ativo só com `confirm`, e aí os controles vão para o perfil Padrão via `ProfileHost::removed`), exportar/importar `.padprofile` (envelope `{format, version, name, description, icon, profile}`, até 256 KiB, id sempre novo), "restaurar padrão" e `compiled_or_default` (um controle nunca fica sem perfil).
- `DeviceRepo`: `upsert_pairing` (parear de novo troca o token e limpa a revogação, mantendo slot, perfil e cor), renomear, revogar, esquecer (apaga o histórico junto), `set_defaults`, `touch_seen`, listar por último visto.
- `ConnectionLog`: `append` e `list` com filtro por dispositivo e paginação (até 500 por página).
- Queries verificadas em compilação (macros `query!`), com os metadados em `.sqlx/` versionados. **Não precisa do `sqlx-cli`**: `scripts/sqlx-prepare.sh` cria um banco só com o schema (`vynpad-dbprep`) e compila o storage contra ele com `SQLX_OFFLINE_DIR`, e as macros gravam os metadados sozinhas. Uma query mudada sem regenerar quebra a compilação offline.

Decisões e desvios:
- **`DeviceStore` fica fora do storage.** A trait é do `vynpad-net`; fazer o storage depender do net traria o TLS inteiro e o net já depende do core. O adaptador (`DeviceRepo` → `DeviceStore`, com `Handle::block_on`, pois o control server já chama a autenticação num `spawn_blocking`) entra na camada de composição do server, junto com o `ProfileHost` do `SlotManager` e o registro no `ConnectionLog` a partir dos eventos do `ControlServer`.
- Os commands Tauri (`get_settings`... `get_connection_log`) esperam a T-506.

Fechamento: o `DeviceStore` (`DeviceStoreAdapter`) e o `ProfileHost` (`SlotRegistry`) vivem no `vynpad-server`; os 15 commands da lista, mais os do server, estão em `apps/desktop/src-tauri/src/commands.rs` (T-506).
