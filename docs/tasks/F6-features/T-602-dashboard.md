# T-602 — Dashboard dos 4 slots

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-203, T-505, T-506 |
| Status | 🟨 Pronta; faltam bateria, ping e "Testar vibração" (dependem de T-305/T-204) e conferir os 4 simuladores no app |

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

## Fechamento
- `features/dashboard`: `DashboardPage` (cabeçalho com status, IP:porta e "Parear novo"; grade de 1 coluna que passa a 2 acima de 860 px) e `SlotCard` (vazio, conectado, reconectando com contador, controle físico, pausado, selo de gyro, select de perfil, menu `…`). `slotCardKind` decide o card e tem teste próprio.
- Backend novo: commands `swap_slots`, `get_slot_profiles` e `set_slot_profile`. A troca de slots leva junto a sessão de controle (`ControlServer::relocate`), a célula do perfil e a `config` do client (que recebe o slot novo); o perfil trocado no card vale até o dispositivo desconectar (o padrão do dispositivo não muda).
- Arrastar e soltar usa HTML5 DnD, então a janela ganhou `dragDropEnabled: false` (o Tauri engolia o evento no Windows); a troca também existe no menu (Trocar de slot → Trocar com Pn).
- O input ao vivo vem do `Gamepad` (T-505), que assina só os slots conectados.
- Fora desta entrega: bateria e ping (precisam das métricas por sessão da T-305) e "Testar vibração" (precisa do rumble da T-204). Os itens entram nos cards quando esses dados existirem.
