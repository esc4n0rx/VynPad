# T-905 — Layout do gamepad (multitouch)

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-901 |
| Status | ✅ Concluída (multitouch validado no aparelho, inclusive com mais de um controle) |

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

## Andamento (2026-10-05)
- **Estado único** (`GamepadState`, `@Volatile`, sem Compose): a tela escreve a cada toque e o motor de envio (T-906) lê na taxa dele.
- **Geometria** (`GamepadGeometry`, código puro, em frações da tela: vale de 5" a 11"): layout Xbox em paisagem, com folga de toque nos botões pequenos; direcional em 4 teclas separadas com leitura em 8 vias; gatilhos analógicos pela altura do dedo.
- **Roteador de toques** (`TouchRouter`, rastreio por `pointerId`): botões que deslizam de um para outro sem levantar o dedo; analógico flutuante que não pula (a base nasce onde o dedo pousa) e fica restrito a um dedo; L3/R3 por duplo toque com o segundo segurado; solta tudo ao abrir o menu.
- **Desenho** num único `Canvas`, com mola por botão e halo na cor do slot (P1 azul…); nada recompõe por toque. Haptics ao apertar, ao deslizar e nos fins de curso (analógico no máximo, gatilho em 50% e 100%). Tela sempre ligada e `requestUnbufferedDispatch` para a menor latência.
- **Menu de pausa** na pílula do topo (também o botão voltar): continuar, desconectar e sair. A pílula mostra o jogador e o PC.
- **Testes na JVM** (`TouchRouterTest`, 11 casos): inclui 5 toques simultâneos (analógico, A, gatilho, direcional e ombro) registrados corretamente.
- **Pendente:** ping e qualidade da conexão no overlay (vêm com o T-906), validar o multitouch real com os dedos e a latência medida (< 2 ms). O editor de layout é o T-909.
