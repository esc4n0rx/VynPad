# T-304 — Ciclo de vida da sessão e reconexão

| Campo | Valor |
|---|---|
| Fase | F3 — Rede |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-301, T-302 |
| Status | ⬜ A fazer |

## Objetivo
Nunca deixar um botão "preso" no jogo e reconectar sem que o jogador perca o controle.

## Escopo
- Watchdog (thread leve ou tarefa com timer de 50 ms):
  - **250 ms** sem UDP → envia o report neutro ao driver (mantém o pad plugado).
  - **5 s** → sessão `Suspended` (UI mostra "Reconectando…").
  - Queda do TLS (ou 15 s sem mensagem de controle) → sessão encerrada; slot `Reserved` por **30 s** → depois, unplug + slot `Free` (PROTOCOLO §9).
- Reconexão: o mesmo `device_token` dentro de 30 s → mesmo slot, novo `session_id`/`udp_key`, sem unplug (o jogo não percebe).
- `bye` explícito → libera o slot na hora.
- Suspensão/retomada do Windows: ao retomar, revalidar o driver e as sessões.
- Eventos `session:state` para a UI.

## Critérios de aceite
- [ ] Teste: segurar "A" no simulador e matar o processo → o botão é solto em ≤ 250 ms.
- [ ] Derrubar o Wi-Fi por 10 s e voltar → mesmo slot, sem o jogo detectar desconexão.
- [ ] Testes automatizados com relógio simulado.

## Fora de escopo
- Métricas (T-305).
