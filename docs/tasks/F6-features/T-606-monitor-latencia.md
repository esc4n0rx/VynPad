# T-606 — Monitor de latência

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-305, T-503 |
| Status | 🟨 Pronta; falta medir os 60 fps com 4 slots e 10 min no app e comparar com o `padgaming-sim bench` |

## Objetivo
Mostrar ao usuário, de forma clara, se a rede está boa para jogar, e ajudar a diagnosticar quando não está.

## Escopo
- Gráfico em tempo real (uPlot, leve para streaming) com a linha de RTT p50/p95 por slot, nas cores dos slots; janelas de 30 s, 2 min e 10 min.
- Tiles por slot: RTT p95, jitter, perda %, pacotes/s, processamento no server, sinal de qualidade.
- Legenda interativa (clicar oculta ou mostra o slot).
- Dicas contextuais: perda > 2% → "Aproxime-se do roteador ou use 5 GHz"; jitter alto → "Desative a economia de bateria do Wi-Fi no celular".
- Pausar e congelar o gráfico; copiar o resumo em texto.
- Gráficos legíveis em dark e light, cores dos slots com contraste adequado, números em `tabular-nums`.

## Critérios de aceite
- [ ] O gráfico roda a 60 fps com 4 slots e a janela de 10 min.
- [ ] Os valores batem com o relatório do `padgaming-sim bench`.

## Fora de escopo
- Exportação de CSV (pós-v1).

## Fechamento
- `features/latency`: `LatencyPage` (janela de 30 s / 2 min / 10 min, pausar, copiar resumo, legenda que esconde slots, tiles por slot com a qualidade e dicas), `LatencyChart` (uPlot: p50 em linha cheia e p95 tracejada, nas cores dos slots, repintando ao trocar o tema) e `insights.ts` (dicas: perda > 2%, jitter > 8 ms, RTT p95 > 60 ms; resumo em texto).
- O histórico (`lib/api/latencyHistory.ts`) guarda 10 min de `metrics:tick` em arrays mutáveis e só avisa por um contador: o gráfico se atualiza sem re-renderizar o React.
- Pausar congela só a vista; o histórico continua crescendo e o gráfico volta a acompanhar ao retomar.
- Os tiles mostram o RTT p95, o jitter, a perda, os pacotes/s e o processamento no server, os mesmos números que o `padgaming-sim bench` reporta (RTT p95 e perda).
- O canvas do uPlot não roda no jsdom: os testes usam um dublê e conferem as chamadas (`setData`, `setSeries`, `destroy`).
