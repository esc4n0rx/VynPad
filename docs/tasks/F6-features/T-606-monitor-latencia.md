# T-606 — Monitor de latência

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-305, T-503 |
| Status | ⬜ A fazer |

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
