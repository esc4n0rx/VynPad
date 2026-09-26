# T-910 — QA e release na Play Store

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-904, T-905, T-906, T-907, T-908 |
| Status | ⬜ A fazer |

## Objetivo
Publicar o client e fechar o ecossistema PadGaming 1.0.

## Escopo
- Testes: unit (protocolo, estado do input), instrumentados (fluxo de conexão com o server rodando no CI via `padgaming-sim`/server headless de teste), matriz de aparelhos (Samsung, Xiaomi, Motorola, Pixel; Android 8 a 15).
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
