# T-403 — Repositórios, serviços e commands

| Campo | Valor |
|---|---|
| Fase | F4 — Persistência |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-402, T-506 |
| Status | ⬜ A fazer |

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
- [ ] Testes de integração dos repositórios com o banco em memória.
- [ ] Editar um perfil na UI altera o comportamento do slot sem reconectar.
- [ ] Não é possível apagar o perfil builtin nem um perfil em uso sem confirmação.

## Fora de escopo
- Telas (F6).
