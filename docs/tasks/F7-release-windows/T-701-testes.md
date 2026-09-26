# T-701 — Estratégia e suíte de testes

| Campo | Valor |
|---|---|
| Fase | F7 — Qualidade e release Windows |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-307 |
| Status | ⬜ A fazer |

## Objetivo
Confiança para evoluir sem quebrar o que já funciona.

## Escopo
| Camada | Ferramenta | Alvo |
|---|---|---|
| Unit Rust | `cargo test`, `proptest` | protocol, mapping, slots, pairing, storage |
| Integração Rust | `tokio::test` + `MockDriver` + sim em processo | handshake, sessão, timeout, reconexão, rumble |
| Fuzz | `cargo-fuzz` | decode de pacotes, parser de mensagens de controle |
| Unit frontend | Vitest + Testing Library | componentes, stores, hooks |
| E2E UI | WebdriverIO + `tauri-driver` (Windows) | onboarding, parear, trocar perfil |
| Manual/HIL | checklist | jogos reais (lista abaixo), ViGEm real |

- Matriz de jogos para o teste manual: um jogo Steam com Steam Input, um Game Pass/UWP, um emulador (RetroArch), um jogo de luta (latência), um FPS (gyro).
- Cobertura: ≥ 80% nos crates de domínio (`cargo-llvm-cov`), relatório no CI.
- `docs/QA-CHECKLIST.md` gerado nesta task.

## Critérios de aceite
- [ ] O CI roda unit + integração + E2E (Windows) em cada PR.
- [ ] Checklist manual executado e registrado antes de cada release.

## Fora de escopo
- Testes do Android (T-910).
