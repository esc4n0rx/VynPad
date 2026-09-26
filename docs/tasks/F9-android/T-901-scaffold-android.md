# T-901 — Scaffold Kotlin + Compose

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | — (iniciar após o M4, conforme o roadmap) |
| Status | ⬜ A fazer |

## Objetivo
Base do app Android alinhada à identidade visual do server.

## Escopo
- `apps/android`: Gradle (Kotlin DSL + version catalog), `minSdk 26`, `targetSdk` atual, Kotlin 2.x, Compose + Material 3 com tema dark customizado (tokens do DESIGN-SYSTEM: accent #0A84FF, superfícies de vidro).
- Módulos: `app`, `core:protocol`, `core:network`, `core:data` (DataStore), `feature:connect`, `feature:gamepad`, `feature:settings`.
- DI com Hilt (ou Koin); navegação com Navigation Compose.
- Telas vazias: Conectar, Gamepad (landscape, tela cheia, imersiva), Configurações.
- i18n PT-BR + EN (`strings.xml`).
- CI: build, testes, ktlint/detekt em um job Android no GitHub Actions.

## Critérios de aceite
- [ ] O app compila e navega entre as telas vazias.
- [ ] CI Android verde.

## Fora de escopo
- Lógica de rede (T-902+).
