# T-608 — Notificações e toasts

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P2 |
| Estimativa | P |
| Depende de | T-602 |
| Status | ⬜ A fazer |

## Objetivo
Avisar sobre eventos relevantes sem ser intrusivo.

## Escopo
- **Toasts internos** (janela aberta): conectado, desconectado, reconectado, pareado, rejeitado, driver com erro.
- **Notificações nativas** (`tauri-plugin-notification`, janela oculta): "Galaxy S24 conectado como P1", pedido de aprovação (com ação, se a plataforma permitir), erro de driver.
- Configurações: ativar/desativar por tipo; não notificar durante jogo em tela cheia (Windows: consultar `SHQueryUserNotificationState`).
- Rate limit: agrupar eventos repetidos (ex.: reconexões em rajada).

## Critérios de aceite
- [ ] Nenhuma notificação aparece com um jogo em tela cheia, quando a opção estiver ativa.
- [ ] 10 reconexões em 5 s geram uma única notificação agrupada.

## Fora de escopo
- Sons personalizados.
