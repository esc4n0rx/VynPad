# T-607 — Tray, autostart, iniciar minimizado

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-503 |
| Status | 🟨 Pronta; falta conferir no Windows: os ícones na bandeja, reiniciar com o autostart ligado, sair pela bandeja sem deixar controles virtuais e a RAM em modo bandeja (< 60 MB) |

## Objetivo
O server fica sempre disponível, sem ocupar a barra de tarefas.

## Escopo
- Ícone na bandeja (Tauri 2 `TrayIconBuilder`) com variantes: ocioso, com N controles conectados (badge), erro de driver.
- Tooltip: "PadGaming — 2 controles conectados".
- Menu: Abrir PadGaming · (lista P1–P4 com nome e ping) · Parear novo dispositivo · Pausar todos os controles · Configurações · Sair.
- Clique esquerdo → abre/foca a janela.
- **Autostart**: `tauri-plugin-autostart` (registro do Run no Windows), argumento `--minimized`.
- **Fechar para a bandeja**: primeira vez mostra o toast "O PadGaming continua rodando na bandeja".
- **Iniciar minimizado**: não cria a janela até o usuário abrir (economiza RAM da webview; avaliar criar a janela sob demanda).
- Menu traduzido (T-504).

## Critérios de aceite
- [ ] Reiniciar o Windows com o autostart ativo → server disponível e janela oculta.
- [ ] Sair pelo tray remove os controles virtuais de forma limpa.
- [ ] RAM em modo tray (sem janela) < 60 MB.

## Fora de escopo
- Tray no Linux (T-803).

## Fechamento
- `tray.rs`: o `TrayModel` (puro e testado) decide ícone (ocioso, com controles, erro de driver), tooltip ("PadGaming — 2 controles conectados"), as linhas dos slots ("P1 · Galaxy da Ana · 25 ms", "P2 · livre", reconectando, pausado) e o item "Pausar todos os controles" (vira "retomar" quando todos estão pausados). Menu e tooltip em PT-BR e EN. `spawn_updater` refaz o ícone e o menu a partir dos eventos do `Core` (slots, métricas, dispositivos, configurações) e a cada 2 s (o driver pode cair sozinho); a latência no menu anda de 5 em 5 ms para não refazer o menu a cada oscilação. Clique esquerdo abre a janela.
- `shell.rs`: a janela principal agora nasce sob demanda (`"create": false` no `tauri.conf.json` e `show_main_window`). **Fechar para a bandeja destrói a janela** e libera a webview; quem mantém o app vivo é o `RunEvent::ExitRequested`, e só "Sair" (código de saída) encerra, pelo caminho normal de parada do `Core` (os controles virtuais saem junto). A primeira vez mostra o aviso "continua rodando na bandeja" na janela e a fecha 2,2 s depois (`tray_hint_shown`, nova configuração).
- "Parear novo dispositivo" e "Configurações" abrem a janela (criando-a) já na tela certa: o pedido fica no backend (`take_pending_navigation`) porque a janela pode nem existir quando o menu é clicado; a UI o lê ao abrir e a cada `tray:navigate`. `/devices?pair=1` abre o modal de pareamento.
- Autostart: `tauri-plugin-autostart` (chave `Run` do Windows), ligado e desligado ao vivo pelo interruptor das Configurações e conferido na abertura. O comando registrado não leva argumento: quem decide se a janela aparece é "Iniciar minimizado" (qualquer abertura), e a flag `--minimized` também funciona numa abertura manual.
- Iniciar minimizado: o app sobe só com a bandeja e o `Core`; a janela é criada quando alguém pedir. Conferido rodando o executável: com `--minimized` não há janela nem erro; sem a flag a janela é criada com o Mica.
- Pendente: a RAM em modo bandeja e os ícones (usam os `.ico` de `icons/tray`) só dá para medir e ver no Windows com o app instalado.
