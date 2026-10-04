# T-304 — Ciclo de vida da sessão e reconexão

| Campo | Valor |
|---|---|
| Fase | F3 — Rede |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-301, T-302 |
| Status | 🟨 Pronta; falta ligar o evento de suspensão do Windows (`system_resumed`) e a UI |

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
- [x] Teste: segurar "A" no simulador e matar o processo → o botão é solto em ≤ 250 ms.
- [x] Derrubar o Wi-Fi por 10 s e voltar → mesmo slot, sem o jogo detectar desconexão.
- [x] Testes automatizados com relógio simulado.

## Fora de escopo
- Métricas (T-305).

## Situação

Pronto, em `vynpad_net::control` (watchdog dentro do `ControlServer`, tarefa de 50 ms) e `SlotManager::neutralize_session`:
- 250 ms sem pacote UDP válido → controle solto, ainda plugado; 5 s → `SessionHealth::Stalled` ("Reconectando…"); o input volta e a sessão retoma sozinha (`Live`). Eventos `ControlEvent::SessionState`.
- A cada 1 s, `SlotManager::tick()` libera as reservas vencidas (30 s depois da queda do TLS): controle desplugado e `ControlEvent::ReservationExpired`.
- Queda do TLS/15 s sem controle → `suspend`; `bye` → `release` (T-302). Reconexão em 30 s volta ao mesmo slot, sem replugar.
- `ControlServer::system_resumed()` solta todos os controles e revalida o driver (`ControlEvent::DriverRechecked`).

Testes: `vynpad-sim/tests/control_handshake.rs` (matar o client solta o "A" em < 450 ms; queda de 10 s volta ao mesmo slot sem desplugar; reserva de 30 s expira com relógio manual; retomada da suspensão) e os unitários de `classify` e `neutralize_session`.

Pendências:
- Chamar `system_resumed()` no evento de suspensão/retomada do Windows (app desktop, T-607).
- `session:state` na UI (T-506/T-602): os eventos já saem do servidor.
- O retorno `Stalled` → `Live` não tem teste de integração (o client simulado reinicia o `seq` ao reconectar o UDP e o anti-replay o recusa); só a lógica de `classify`.
