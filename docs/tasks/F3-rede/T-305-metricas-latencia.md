# T-305 — Métricas de latência

| Campo | Valor |
|---|---|
| Fase | F3 — Rede |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-301 |
| Status | 🟨 Pronta; falta validar o RTT (±1 ms com 20 ms ± 5 ms) com `netem`/`clumsy` e medir o custo por pacote em release |

## Objetivo
Medir continuamente a qualidade de cada conexão, alimentando o monitor de latência (T-606) e o benchmark (T-702).

## Escopo
- Por slot:
  - **RTT** via Ping/Pong (o client pinga a 4 Hz; o server também calcula ao receber o próximo ping com o eco).
  - **Jitter** (desvio do intervalo entre pacotes de input).
  - **Perda** (lacunas em `seq`) e **fora de ordem**.
  - **Taxa** (pacotes/s).
  - **Processamento no server** (recv → `driver.update` concluído).
  - Bateria do celular (campo do InputState).
- `hdrhistogram` com janelas de 1 s e de 60 s; percentis p50/p95/p99.
- Evento `metrics:tick` a 4 Hz (só quando a UI está visível; caso contrário, 0,2 Hz para o tray).
- Classificação de qualidade: Excelente / Boa / Instável / Ruim, com limiares configuráveis, usada nos ícones.

## Critérios de aceite
- [ ] Com `netem`/`clumsy` simulando 20 ms ± 5 ms, o RTT mostrado fica dentro de ±1 ms.
- [ ] O overhead das métricas no hot path é < 1 µs por pacote.

## Fora de escopo
- Gráficos (T-606).

## Fechamento
- `vynpad-net::input::SessionMetrics` (dentro do `SessionActivity` de cada sessão): perda (lacunas de `seq`), pacotes repetidos ou fora de ordem, jitter (média móvel do desvio do intervalo entre inputs, 1/16 como no RFC 3550), bateria, tempo de processamento e RTT, os dois últimos em janelas deslizantes de 60 s (`RollingHistogram`: um histograma por segundo) com p50/p95/p99. Tudo atômico; o teste `per_packet_cost_is_negligible` confere < 1 µs por pacote e `handling_datagrams_does_not_allocate` continua verde.
- **RTT pelo canal de controle**: o `Pong` UDP carrega o eco para o client medir, e o server não tem como saber o RTT por ali sem mudar o protocolo v1 (congelado). Em vez disso o server manda um `ping` TLS a cada 250 ms (`ControlConfig::rtt_probe_every`) e mede até o `pong` do client. É o RTT do caminho TCP, que numa LAN acompanha o do UDP; o client Android precisa responder `pong` na hora (PROTOCOLO §5).
- `vynpad-server::MetricsCollector` amostra as sessões vivas (taxa e perda numa janela de 10 s), classifica a qualidade (`Quality`: Excelente, Boa, Instável, Ruim, com `QualityThresholds` padrão em RTT p95, jitter e perda) e o `Core` publica `CoreEvent::Metrics` a 4 Hz com a janela à vista e a 0,2 Hz escondida (`Core::set_ui_visible`).
- App: evento `metrics:tick`, commands `get_metrics` e `set_ui_visible`, store `useMetrics` e o dashboard (T-602) mostra bateria, latência e o selo de qualidade em cada card. Os limiares ainda não são configuráveis pela UI.
