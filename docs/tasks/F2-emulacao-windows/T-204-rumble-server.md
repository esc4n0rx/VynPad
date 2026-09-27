# T-204 — Rumble: driver → client

| Campo | Valor |
|---|---|
| Fase | F2 — Emulação Windows |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-202, T-301 |
| Status | ⬜ A fazer |

## Objetivo
Repassar a vibração pedida pelo jogo para o celular do slot correspondente.

## Escopo
- Thread `rumble-tx` consome `driver.rumble_events()`.
- Resolve slot → sessão → endereço UDP do client + `SessionCipher`.
- Envia o pacote `0x20` com um `rumble_id` novo quando o valor muda e retransmite o estado atual a cada 100 ms até o client devolver `rumble_ack_id` igual ao `rumble_id` atual (PROTOCOLO §6.4 e §8). Isso vale também para `0,0`, o que garante o desligamento mesmo com perda.
- Configuração por perfil: intensidade (0–200%) e desligar rumble.
- Teste de vibração pela UI (botão "Testar vibração" no card do slot).

## Critérios de aceite
- [ ] Com o simulador, o rumble do "XInput Test" aparece no log do `padgaming-sim`.
- [ ] Com 10% de perda simulada, o motor nunca fica "preso" ligado.
- [ ] O botão de teste funciona.

## Fora de escopo
- Vibração no Android (T-907).
