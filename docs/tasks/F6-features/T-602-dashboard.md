# T-602 — Dashboard dos 4 slots

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-203, T-505, T-506 |
| Status | ⬜ A fazer |

## Objetivo
Visão imediata de quem está jogando em cada controle.

## Escopo
- Grade 2×2 (1 coluna em janelas estreitas) com cards de vidro P1–P4:
  - **Vazio**: gamepad em contorno tracejado, "Aguardando conexão", botão "Parear dispositivo".
  - **Conectado**: gamepad `live` na cor do slot, nome e modelo do dispositivo, bateria, ping com indicador de qualidade, perfil ativo (select para trocar), badge "Gyro".
  - **Reconectando**: pulso âmbar + contador.
  - **Ocupado por controle físico** (T-205): card informativo.
- Ações do card (menu `…`): Testar vibração, Trocar de slot (arrastar o card para outro slot também funciona), Pausar, Desconectar, Ver latência.
- Cabeçalho: status do server, IP:porta, botão "Parear novo".
- Animação de "acender" quando um controle conecta.

## Critérios de aceite
- [ ] Com 4 simuladores, os 4 cards mostram o input ao vivo sem travar a UI.
- [ ] Trocar o perfil no card aplica na hora.
- [ ] Arrastar P1 → P3 troca os slots.

## Fora de escopo
- Gráficos detalhados (T-606).
