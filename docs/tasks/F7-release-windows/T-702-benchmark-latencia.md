# T-702 — Benchmark de latência e performance

| Campo | Valor |
|---|---|
| Fase | F7 — Qualidade e release Windows |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-305, T-307 |
| Status | ⬜ A fazer |

## Objetivo
Provar as metas de RNF-01 a RNF-04 com números reproduzíveis.

## Escopo
- **Server**: `padgaming-sim bench --clients 4 --rate 250 --duration 300s` → p50/p95/p99 do processamento; alvo p99 < 1 ms.
- **Ponta a ponta sem celular**: loopback de medição: o sim envia um pulso e, na mesma máquina, lê o `XInputGetState` até ver a mudança; mede de envio até visível no XInput.
- **Ponta a ponta com celular** (após F9): câmera 240 fps filmando o toque e um indicador na tela (procedimento documentado); alvo p95 < 10 ms em 5 GHz.
- **Recursos**: CPU (ocioso e com 4 controles), RAM (janela aberta, só tray), tempo de boot.
- Relatório versionado em `docs/benchmarks/AAAA-MM-DD.md`; o CI roda o bench do server e falha em regressão > 20%.

## Critérios de aceite
- [ ] Todas as metas do SPEC §5 medidas e documentadas.
- [ ] Gate de regressão ativo no CI.

## Fora de escopo
- Otimizações (abrir tasks específicas se alguma meta falhar).
