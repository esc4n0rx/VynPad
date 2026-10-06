# T-901 — Scaffold Kotlin + Compose

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | — (iniciar após o M4, conforme o roadmap) |
| Status | 🟨 Em andamento (subtask A: scaffold e design system) |

> **Atualização (D-26, 2026-10-04):** o client passa a ser **Compose Multiplatform** (Android primeiro, iOS depois), com o núcleo de rede e protocolo em **Rust via UniFFI**. Esta task começa por uma **prova de conceito** que valida a arquitetura antes do scaffold final; os módulos abaixo viram `shared/` (commonMain) + `androidApp/`, e `core:protocol` e `core:network` passam a ser o crate Rust. Ver "Prova de conceito" no fim.

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

## Andamento (2026-10-05)
A prova de conceito abaixo foi **dispensada**: seguimos direto para o scaffold final, em subtasks aprovadas uma a uma (A scaffold e design system, B núcleo Rust, C descoberta, D pareamento, E gamepad, F motor de envio, G rumble e gyro, H personalização, I QA e release).

- **A (feita):** `apps/android` com `shared/` (Compose Multiplatform) e `androidApp/`; tema com os tokens do desktop, Inter, mola 380/32, `GlassCard`, botão, toggle, segmented, ícones, haptics (`Haptics`), telas Conectar, Ajustes e Gamepad (invólucro imersivo) e i18n PT/EN. Versões: AGP 9.4.1, Kotlin 2.4.20, Compose Multiplatform 1.12.1, Gradle 9.6.0, `compileSdk` 37, `minSdk` 26. Ainda sem ktlint/detekt e sem job de CI.

## Prova de conceito (dispensada)
Decide a arquitetura com números. Só vale seguir com Compose Multiplatform + Rust se tudo abaixo passar; senão, plano B: Flutter.
1. **UI:** uma tela de gamepad em Compose a 120 fps, com as animações do server (glow na cor do slot, molas equivalentes a stiffness 380 / damping 32, vidro e tema dark com os tokens do design system).
2. **Núcleo Rust via UniFFI:** crate `vynpad-client-core` (reaproveitando `vynpad-protocol`) enviando input a 250 Hz para o `dev_server`, com build integrado ao Gradle (`cargo-ndk`) e checagem dos test vectors.
3. **Medição:** toque até pacote UDP (`vynpad-sim bench` e câmera de 240 fps, como no T-702), com `requestUnbufferedDispatch`, e fluidez num celular de gama média. Meta: p95 < 10 ms de ponta a ponta.
4. **Veredito** registrado no `DECISOES.md` (confirma D-26 ou aciona o plano B).

## Perguntas abertas para o início
- "Multiplataforma" = Android + iOS (assumido) ou também tablets, Chromebooks ou PC?
- Há Mac e iPhone para testar o iOS? Sem eles, o iOS fica só desenhado (conta Apple de US$ 99/ano).
