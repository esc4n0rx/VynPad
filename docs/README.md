# PadGaming — Documentação do Projeto

Gamepad virtual: um **client Android** envia o input pela rede local para um **server desktop (Windows / Linux)**, que expõe esse input como um **controle Xbox 360 nativo**, compatível com qualquer jogo XInput.

## Como navegar

| Documento | Conteúdo |
|---|---|
| [00-visao-geral/SPEC.md](00-visao-geral/SPEC.md) | Especificação do produto: objetivos, escopo, requisitos, metas |
| [00-visao-geral/DECISOES.md](00-visao-geral/DECISOES.md) | Registro das decisões técnicas e de produto (ADR-lite) |
| [00-visao-geral/ARQUITETURA.md](00-visao-geral/ARQUITETURA.md) | Arquitetura, crates, fluxo de dados, threading |
| [00-visao-geral/PROTOCOLO.md](00-visao-geral/PROTOCOLO.md) | Rascunho do protocolo client ↔ server (finalizado na T-101) |
| [00-visao-geral/DESIGN-SYSTEM.md](00-visao-geral/DESIGN-SYSTEM.md) | Identidade visual estilo macOS Sonoma, tokens, componentes |
| [00-visao-geral/ROADMAP.md](00-visao-geral/ROADMAP.md) | Fases, marcos, dependências e **quadro de tasks** |
| [tasks/](tasks/) | Uma task por arquivo, organizadas por fase |

## Fases

| Fase | Pasta | Tema |
|---|---|---|
| F0 | [tasks/F0-fundacao](tasks/F0-fundacao) | Monorepo, scaffold Tauri, CI, logging |
| F1 | [tasks/F1-protocolo](tasks/F1-protocolo) | Protocolo binário, crate de protocolo, pareamento seguro |
| F2 | [tasks/F2-emulacao-windows](tasks/F2-emulacao-windows) | ViGEmBus, 4 slots, rumble |
| F3 | [tasks/F3-rede](tasks/F3-rede) | UDP/TCP, descoberta mDNS, sessões, simulador |
| F4 | [tasks/F4-persistencia](tasks/F4-persistencia) | SQLite, schema, repositórios |
| F5 | [tasks/F5-ui-shell](tasks/F5-ui-shell) | Design system, janela com vibrancy, layout, i18n |
| F6 | [tasks/F6-features](tasks/F6-features) | Onboarding, dashboard, dispositivos, mapeamento, tray |
| F7 | [tasks/F7-release-windows](tasks/F7-release-windows) | Testes, benchmark, instalador, auto-update |
| F8 | [tasks/F8-linux](tasks/F8-linux) | Backend uinput, empacotamento Linux |
| F9 | [tasks/F9-android](tasks/F9-android) | Client Android (Kotlin + Compose) |

## Template de task

Toda task segue [tasks/_TEMPLATE.md](tasks/_TEMPLATE.md).
