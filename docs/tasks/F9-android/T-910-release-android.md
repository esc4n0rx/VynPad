# T-910 — QA e release na Play Store

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-904, T-905, T-906, T-907, T-908 |
| Status | 🟡 Em andamento (build, CI, lint e documentação prontos; falta o que depende do Play Console e de aparelhos) |

## Objetivo
Publicar o client e fechar o ecossistema VynPad 1.0.

## Escopo
- Testes: unit (protocolo, estado do input), instrumentados (fluxo de conexão com o server rodando no CI via `vynpad-sim`/server headless de teste), matriz de aparelhos (Samsung, Xiaomi, Motorola, Pixel; Android 8 a 15).
- Medição ponta a ponta com câmera (procedimento da T-702) e registro em `docs/benchmarks/`.
- Play Console: listing PT-BR/EN, screenshots, política de privacidade (sem coleta de dados), declaração de foreground service, App Bundle assinado (Play App Signing).
- Faixas: interno → beta fechado → produção.
- Atualizar o onboarding do server (T-601) com o link/QR da Play Store.
- Crash reporting opcional e opt-in (ou nenhum, conforme a política de privacidade).

## Critérios de aceite
- [ ] Taxa de crash < 0,5% no beta.
- [ ] Meta de latência p95 < 10 ms confirmada em pelo menos 3 aparelhos.
- [ ] App publicado em produção.

## Fora de escopo
- iOS.

## Entregue (2026-10-06)
- Build de release com R8 e regras para JNA/UniFFI, kotlinx.serialization e ML Kit; App Bundle (`bundleRelease`).
- Assinatura por variáveis de ambiente (`VYNPAD_KEYSTORE*`), sem segredos no repositório.
- ktlint (`.editorconfig`, binário fixado por hash no CI), Android Lint (um erro real corrigido: `windowLayoutInDisplayCutoutMode` exige API 27) e job `android` no `ci.yml`.
- `data_extraction_rules.xml` (sem backup nem transferência de dados).
- Procedimento, "Segurança dos dados", política de privacidade e texto da loja em `docs/android/PUBLICACAO.md`.

## Pendente
- Decidir e implementar o serviço em primeiro plano (pendência da T-906) e o tipo a declarar no Play Console.
- Conta no Play Console, chave de upload, política de privacidade hospedada, screenshots e faixas (interno → fechado → produção).
- Testes instrumentados no CI contra o server de teste e a matriz de aparelhos.
- Medição de latência ponta a ponta com câmera em 3 aparelhos (T-702) e registro em `docs/benchmarks/`.
- Onboarding do desktop com o link da Play Store (T-601).
