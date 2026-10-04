# T-608 — Notificações e toasts

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P2 |
| Estimativa | P |
| Depende de | T-602 |
| Status | 🟨 Pronta; falta ver as notificações do sistema no Windows e conferir com um jogo em tela cheia de verdade |

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

## Fechamento
- Quem decide é o backend (`notify.rs`), porque a janela pode estar fechada: **janela à vista → toast** (o evento `notice` com texto já traduzido), **sem janela → notificação nativa** (`tauri-plugin-notification`). Os eventos vêm do `Core`: conectou, reconectou (voltou em até 30 s), desconectou (só quando o celular fechou ou a conexão caiu; desconectar pela UI, revogar e fechar o app não avisam), pareou, conexão recusada (com o motivo) e driver que para ou volta.
- **Agrupamento** (`Coalescer`, puro e testado): eventos iguais (mesmo tipo e mesmo dispositivo) esperam 1,5 s sem novidade, ou no máximo 5 s, e saem como um aviso com a contagem ("Galaxy reconectou 10 vezes em P1"). Dez reconexões em 5 s geram um aviso só.
- **Tela cheia**: `vynpad_net::user_state::fullscreen_busy` consulta `SHQueryUserNotificationState` (ocupado, tela cheia D3D e modo apresentação) e, com a opção ligada, nenhum aviso aparece.
- Configurações → Notificações: conexões, pareamento, driver e "silêncio durante jogos em tela cheia" (padrão: tudo ligado).
- O pedido de aprovação de um dispositivo novo traz a janela para a frente (o diálogo mora nela), exceto com um jogo em tela cheia e a opção ligada. As notificações nativas do Windows não têm botão de ação como no celular, então não há "Permitir" na própria notificação.
- Fora desta entrega: toasts para a perda de sinal ("reconectando", que o card do dashboard já mostra).
