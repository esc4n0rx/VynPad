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
- Envia o pacote `0x20` quando o valor muda; repete a cada 100 ms enquanto ≠ 0 até receber `rumble_ack_seq` ≥ `seq` enviado.
- Ao zerar, envia `0,0` três vezes (garante o desligamento mesmo com perda).
- Configuração por perfil: intensidade (0–200%) e desligar rumble.
- Teste de vibração pela UI (botão "Testar vibração" no card do slot).

## Critérios de aceite
- [ ] Com o simulador, o rumble do "XInput Test" aparece no log do `padgaming-sim`.
- [ ] Com 10% de perda simulada, o motor nunca fica "preso" ligado.
- [ ] O botão de teste funciona.

## Fora de escopo
- Vibração no Android (T-907).
