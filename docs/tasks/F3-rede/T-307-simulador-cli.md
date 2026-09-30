# T-307 — Simulador CLI `padgaming-sim`

| Campo | Valor |
|---|---|
| Fase | F3 — Rede |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-102 (T-302/T-303 para os modos completos) |
| Status | 🟨 M1 atingido; faltam `discover` (T-303), os subcomandos `pair`/`connect` sem `--insecure-dev` e `--interactive` |

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

## Situação

Pronto (testado, incluindo um teste de ponta a ponta contra o `InputServer` real):
- `connect --insecure-dev` com `--pattern circle|buttons|stress`, `--script arquivo.toml` (passos com botões, analógicos, gatilhos e gyro, com `loop`) e `--gamepad` (controle físico via `gilrs`, atrás da feature `gamepad`).
- `bench --clients N --rate R --duration D [--json arquivo]`: RTT por client e total (p50/p95/p99/máx), pings sem resposta e tempo de processamento no server; relatório em texto e em JSON.
- Falhas: `--loss`, `--delay`, `--jitter`, `--reorder` (sentido client → server; `--seed` deixa reproduzível).
- Mostra o rumble recebido (o server ainda não envia: T-204).
- Server de desenvolvimento: `cargo run -p padgaming-net --example dev_server` (reabre sessões ociosas, para rodar o simulador várias vezes seguidas). `cargo run -p padgaming-driver --example xinput_watch` mostra o controle virtual no XInput.
- **M1 verificado** nesta máquina com o ViGEmBus real: o simulador desenhou círculos e o controle virtual (XInput índice 2, atrás de dois controles já presentes) acompanhou os analógicos e os gatilhos.

Pendente:
- `discover` (T-303) e `pair` (T-302): existem como subcomandos que explicam a dependência.
- `--interactive` (TUI com `ratatui`): não implementado.
- `--gamepad` compila e roda, mas **não foi validado com um controle físico**: o `gilrs` não listou nenhum controle nesta máquina. O critério de jogar um jogo real com um controle em outro PC depende disso e da T-302.

O canal de controle do client já existe como biblioteca (`padgaming_sim::control`: TLS 1.3 com fingerprint fixado ou TOFU, `hello`/`welcome`, `ping`/`bye`) e é exercitado pelos testes da T-302. Falta expô-lo na CLI (`pair` salvando o `device_token`, `connect` sem `--insecure-dev`).
