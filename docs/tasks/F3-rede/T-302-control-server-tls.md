# T-302 — Servidor de controle TLS

| Campo | Valor |
|---|---|
| Fase | F3 — Rede |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-103, T-203 |
| Status | ✅ Concluída |

## Objetivo
Aceitar conexões dos clients, autenticar, criar sessões e distribuir as chaves UDP.

## Escopo
- Listener Tokio na porta 47800 com `tokio-rustls`.
- Handshake: `hello` → valida `proto_ver` → `auth` via `PairingService` → aloca slot (T-203) → gera `session_id` + `udp_key` → `welcome`.
- Novo pareamento aprovado → emite `device_token`, persiste o dispositivo e dispara o evento `devices:paired`.
- Opção "Pedir confirmação para novos dispositivos" (popup na UI com timeout de 30 s), além do QR/PIN.
- Keepalive conforme PROTOCOLO §5.3 (`ping` após 5 s sem enviar; fecha após 15 s sem receber). A queda do TLS encerra a sessão UDP (a chave é descartada) e o slot fica reservado por 30 s (PROTOCOLO §9, T-304).
- Mensagem `config` enviada quando o perfil muda (ex.: gyro ligado → client passa a enviar 0x02).
- Limites: máximo de 16 conexões pendentes; `hello` em até 10 s; frames de até 16 KiB (PROTOCOLO §5.1).

## Critérios de aceite
- [x] Teste de integração: `padgaming-sim` faz o handshake completo e envia input.
- [x] Um client com token revogado recebe `reject{revoked}`.
- [x] Um `proto_ver` incompatível recebe `reject{version}` com a mensagem clara no client.

## Fora de escopo
- UI de pareamento (T-603).

## Situação

Pronto, em `padgaming_net::control` (`ControlServer`), testado por `padgaming-sim/tests/control_handshake.rs` (15 cenários, com TLS e UDP reais em loopback):
- Handshake `hello` → versão → `PairingService` → slot (`SlotManager::assign`) → `session_id`/`udp_key` aleatórios registrados no `InputServer` → `welcome`. `reject` com `bad_request`, `bad_auth`, `locked`, `revoked`, `no_slot` e `version` (com `supported_versions`).
- `device_token` no `welcome` do primeiro pareamento; o evento `ControlEvent::Paired` sai depois do `welcome` (a persistência é do `DeviceStore`, T-403).
- Confirmação de novos dispositivos: `set_require_approval` + `set_approver`. O pedido só é feito depois de a credencial (QR/PIN) valer e antes de gravar o dispositivo, por isso o `PairingService` ganhou `authenticate_with(hello, peer, approve)`. Sem resposta em 30 s, ou sem UI, recusa com `bad_auth`.
- Keepalive (`ping` após 5 s sem enviar, fecha após 15 s sem receber), `config` via `send_config`, `bye` via `end_session` e `shutdown`.
- TLS caiu ou ficou mudo → `SlotManager::suspend` (reserva de 30 s); `bye` do client, kick, revogação e shutdown → `release`. Uma conexão nova do mesmo dispositivo derruba a antiga (`kicked`) e volta ao mesmo slot.
- Limites: 16 conexões pendentes (as demais são fechadas na hora), TLS + `hello` em 10 s, frames de 16 KiB, escrita com prazo de 5 s.

Fica para depois:
- T-304: o `tick()` periódico que libera as reservas vencidas e a suspensão por silêncio do UDP.
- T-403: `DeviceStore` sobre o SQLite; T-603: a UI do pedido de confirmação.
- T-204: `rumble_enabled` no `config` e o envio de vibração.
- Regras de rate limit por IP para `device_token` inválido (hoje só o PIN tem bloqueio).
