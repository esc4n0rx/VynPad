# T-906 — Motor de envio de baixa latência

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-902, T-905 |
| Status | 🟨 Em andamento (subtask F: envio, RTT, WifiLock e reconexão prontos e validados no PC real; faltam o serviço em primeiro plano e a medição de latência p95) |

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

## Andamento (2026-10-05)
- **Motor de envio em Rust** (`InputPump` no `vynpad-client-core`), duas threads sobre a mesma sessão cifrada: o **envio** acorda na hora em que o estado muda (condvar, sem relógio fixo), respeita o teto de taxa do server (`send_rate_hz`, até 250 Hz), repete o estado a **10 Hz** como keepalive e manda `ping` a **4 Hz**; a **recepção** abre pong (RTT de rede = RTT − `server_proc_us`) e rumble (que é confirmado no `rumble_ack_id` seguinte). Nada aloca por pacote: o selo usa buffer de pilha.
- **Testes contra o server de input real** (`tests/pump.rs`): o estado chega ao pad virtual, o neutro também, o keepalive cadencia o envio, mudanças a milhares por segundo ficam sob o teto de 250 Hz, o RTT é medido, a vibração volta e o `stop` é limpo e único.
- **Android:** o estado do controle (`GamepadState`) empurra cada mudança ao motor sem recompor (`GamepadSink`); bateria a cada 30 s no pacote; `WifiLock` de baixa latência (`FULL_LOW_LATENCY`, API 29+) durante a sessão; RTT suavizado na faixa de conexão e na pílula do gamepad (verde < 20 ms, amarelo < 50 ms); se o contador de pacotes acabar (`seq_exhausted`) ou o socket falhar, a sessão reconecta sozinha com backoff.
- **Validado no aparelho com o desktop real** (2026-10-05): botão A (`0x1000`) e analógico (`lx`/`ly` positivos para direita e cima, com valores intermediários) chegaram ao controle virtual lido pelo XInput do Windows.
- **Latência da rede de teste:** celular em Wi-Fi 6, 5,2 GHz, −45 dBm, mas `ping` do PC até ele de 6 a 142 ms (23% de retransmissões): a latência vem do caminho do PC (adaptador, roteador ou interferência), não do app. O critério de RTT p95 < 8 ms exige uma rede em condições (Ethernet no PC ou 5 GHz sem interferência).
- **Pendente:** serviço em primeiro plano ("Conectado como P1"), medição formal do RTT p95 e da alocação com o Memory Profiler, e vibração do jogo no aparelho (subtask G).
