# T-305 — Métricas de latência

| Campo | Valor |
|---|---|
| Fase | F3 — Rede |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-301 |
| Status | ⬜ A fazer |

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
