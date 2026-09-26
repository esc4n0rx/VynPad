# T-905 — Layout do gamepad (multitouch)

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-901 |
| Status | ⬜ A fazer |

## Objetivo
Um controle na tela que seja gostoso de usar: preciso, responsivo, bonito.

## Escopo
- Tela landscape, imersiva (esconde as barras do sistema), mantém a tela ligada.
- Layout padrão X360: sticks analógicos (flutuantes ou fixos), D-pad, ABXY, LB/RB, LT/RT **analógicos** (arrastar vertical = pressão), Back/Start/Guide, clique L3/R3 (duplo toque ou botão dedicado).
- Multitouch real: `pointerInput` com rastreamento por `pointerId`; até 10 toques simultâneos; deslizar de um botão para outro (como em controles físicos).
- Haptic curto ao pressionar (configurável).
- Visual: botões com a cor do slot atribuído (P1 azul etc.), feedback visual imediato no toque, overlay com status (slot, ping e qualidade da conexão).
- Estado de input em uma estrutura mutável única (sem recomposição por toque) → lido pelo motor de envio (T-906).
- Menu de pausa (botão discreto): desconectar, configurações, recentralizar gyro.

## Critérios de aceite
- [ ] 5 toques simultâneos (stick + 2 botões + gatilho + D-pad) registrados corretamente.
- [ ] Latência do toque até o estado atualizado < 2 ms (medida no app).
- [ ] Layout funciona de 5" a tablets de 11".

## Fora de escopo
- Editor de layout (T-909).
