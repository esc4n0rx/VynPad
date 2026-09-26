# T-607 — Tray, autostart, iniciar minimizado

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-503 |
| Status | ⬜ A fazer |

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
