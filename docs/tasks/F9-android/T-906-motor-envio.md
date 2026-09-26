# T-906 — Motor de envio de baixa latência

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-902, T-905 |
| Status | ⬜ A fazer |

## Objetivo
Levar o toque até o PC o mais rápido possível, dentro da meta p95 < 10 ms.

## Escopo
- Thread dedicada (`Process.THREAD_PRIORITY_URGENT_DISPLAY` ou similar) com `DatagramSocket` conectado.
- Envio **imediato quando o estado muda** (sinalizado pelo layout) com limite de 250 Hz, + keepalive a 10 Hz.
- Ping a 4 Hz; exibir RTT no overlay.
- `WifiManager.WifiLock` em modo `WIFI_MODE_FULL_LOW_LATENCY` (API 29+) / `HIGH_PERF` durante a sessão.
- Foreground service enquanto estiver conectado (evita que o sistema mate a conexão), com notificação "Conectado como P1".
- Reconexão automática (TLS + nova sessão) com backoff; o overlay mostra "Reconectando…".
- Relatório da bateria no pacote.

## Critérios de aceite
- [ ] RTT p95 < 8 ms em Wi-Fi 5 GHz num aparelho intermediário.
- [ ] Nenhuma alocação por pacote (verificado com o Memory Profiler).
- [ ] Bloquear e desbloquear a tela reconecta sozinho.

## Fora de escopo
- Benchmark ponta a ponta com câmera (T-702).
