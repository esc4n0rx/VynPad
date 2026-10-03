# T-505 — Gamepad SVG ao vivo + ícones

| Campo | Valor |
|---|---|
| Fase | F5 — UI Shell |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-501 |
| Status | 🟨 Pronta; falta medir os 60 fps com 4 gamepads no app e ver os ícones da bandeja no Windows (a bandeja é a T-607) |

## Objetivo
O coração visual do app: um controle X360 desenhado com carinho, que reage ao input em tempo real e serve de interface no editor de mapeamento.

## Escopo
- SVG do controle X360 em estilo "flat glass" (contornos suaves, gradientes sutis), com grupos nomeados: `btn-a/b/x/y`, `lb/rb`, `lt/rt`, `dpad-up/down/left/right`, `start/back/guide`, `stick-left/right` (base + cap), `ls-click/rs-click`.
- Componente `<Gamepad state={XReport} slotColor mode="live|interactive|mini" />`:
  - `live`: botões acendem na cor do slot; sticks deslocam o cap; gatilhos preenchem de baixo para cima.
  - `interactive`: hover/click seleciona o elemento (usado pela T-604), com tooltip do nome.
  - `mini`: versão compacta para cards, tray e listas.
- Atualização via `requestAnimationFrame`, lendo de um store fora do React (evita re-render a 60 Hz).
- Ícones: app (squircle com gradiente), dispositivo (celular + gamepad) nas variantes connected/idle/pairing/offline, indicadores P1–P4, ícones do tray (normal, com controles conectados, erro) em `.ico` multi-resolução.

## Critérios de aceite
- [ ] Com o simulador em `--pattern circle`, a animação fica fluida (60 fps) com 4 gamepads na tela.
- [ ] Todos os elementos são selecionáveis no modo interactive.
- [ ] Ícones nítidos a 100%, 150% e 200% de escala.

## Fora de escopo
- Lógica do editor (T-604).
