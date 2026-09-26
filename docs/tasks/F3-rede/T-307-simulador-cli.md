# T-307 — Simulador CLI `padgaming-sim`

| Campo | Valor |
|---|---|
| Fase | F3 — Rede |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-102 (T-302/T-303 para os modos completos) |
| Status | ⬜ A fazer |

## Objetivo
Testar o server ponta a ponta muito antes do app Android existir, e servir de base para os testes automatizados e o benchmark.

## Escopo
- Binário Rust multiplataforma (`clap`).
- Subcomandos:
  - `discover`: lista servers via mDNS/broadcast.
  - `pair --uri "padgaming://pair?..."` ou `pair --pin 123456 --host ...`: salva o `device_token` em um arquivo local.
  - `connect`: modos de input:
    - `--script arquivo.toml` (sequência de estados com tempos),
    - `--pattern circle|buttons|stress`,
    - `--gamepad` (lê um controle físico local via `gilrs` e repassa: ótimo para testes reais em jogos),
    - `--interactive` (teclado → botões, TUI com `ratatui`).
  - `bench --clients 4 --rate 250 --duration 60s`: carga + relatório de RTT.
- Opções de falha: `--loss 10%`, `--delay 20ms`, `--jitter 5ms`, `--reorder`.
- Exibe o rumble recebido.
- `--insecure-dev` para o M1.

## Critérios de aceite
- [ ] O modo `--gamepad` permite jogar um jogo real usando um controle físico em outro PC via PadGaming.
- [ ] `bench` gera um relatório em JSON e texto.

## Fora de escopo
- GUI.
