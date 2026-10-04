# T-701 — Estratégia e suíte de testes

| Campo | Valor |
|---|---|
| Fase | F7 — Qualidade e release Windows |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-307 |
| Status | 🟨 Em andamento |

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
- [ ] O CI roda unit + integração + E2E (Windows) em cada PR. Os jobs `coverage` e `e2e` estão no `ci.yml`, mas ainda não rodaram no GitHub (só localmente).
- [ ] Checklist manual executado e registrado antes de cada release. O `docs/QA-CHECKLIST.md` existe; a primeira execução fica para o primeiro release.

## Implementação
- **Propriedades (`proptest`)**: `vynpad-protocol/tests/properties.rs` (ida e volta de todo payload nos dois sentidos, qualquer bit adulterado é recusado, replay, bytes aleatórios, janela anti-replay, `rumble_id`, URI do QR), `vynpad-core/tests/properties.rs` (invariantes da tabela de slots após qualquer sequência de operações) e o que o `vynpad-mapping` já tinha. Pairing e storage ficaram só com testes de unidade e integração (cobertura de 95% no storage).
- **Integração**: o `vynpad-sim` ganhou o `pair` de verdade (PIN ou URI do QR, com pinning, `--hold` para manter a sessão e `--save` para guardar o token), testado contra o `Core` inteiro (`vynpad-server/tests/core.rs`).
- **Cobertura**: `cargo-llvm-cov` no job `coverage`; `scripts/coverage-gate.mjs` exige 80% de linhas nos crates de domínio (hoje: protocol 96%, mapping 100%, core 99%, storage 95%, server 95%, net 87%). Ficam fora do gate o `vynpad-driver` (60%: a parte do ViGEmBus só roda com o driver, nos testes `#[ignore]`), o simulador e o `dbprep`. A tabela vai para o resumo do job.
- **E2E da UI**: `e2e/` (WebdriverIO + `tauri-driver` + `msedgedriver` da versão do WebView2). Três cenários: introdução, pareamento por PIN (a UI mostra o PIN, o simulador o digita, e um PIN errado é recusado) e troca de perfil. Rodam em 13 s. O app tem dois ganchos de teste: `VYNPAD_DATA_DIR` (pasta de dados) e `VYNPAD_E2E` (portas 47900–47902, sem mDNS, sem procurar atualização).
- **Fuzz**: já existia (`cargo-fuzz` no `fuzz.yml` e o estresse por mutação).
- **Manual**: `docs/QA-CHECKLIST.md` com a matriz de jogos, instalação, pareamento, atualização e a tabela de registro das execuções.

## Decisões
- O script do E2E se chama `e2e`, e não `test`: o `pnpm test` da raiz roda o `test` de todos os pacotes e quebraria no Linux.
- Os cenários compartilham a pasta de dados (o primeiro conclui a introdução), então precisam rodar em ordem.
- Seletor de texto no WebdriverIO: `*=` só acha links; usamos XPath (`byText`). Menus do Radix, pelo teclado.

## Fora de escopo
- Testes do Android (T-910).
