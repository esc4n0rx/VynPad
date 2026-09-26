# T-302 — Servidor de controle TLS

| Campo | Valor |
|---|---|
| Fase | F3 — Rede |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-103, T-203 |
| Status | ⬜ A fazer |

## Objetivo
Aceitar conexões dos clients, autenticar, criar sessões e distribuir as chaves UDP.

## Escopo
- Listener Tokio na porta 47800 com `tokio-rustls`.
- Handshake: `hello` → valida `proto_ver` → `auth` via `PairingService` → aloca slot (T-203) → gera `session_id` + `udp_key` → `welcome`.
- Novo pareamento aprovado → emite `device_token`, persiste o dispositivo e dispara o evento `devices:paired`.
- Opção "Pedir confirmação para novos dispositivos" (popup na UI com timeout de 30 s), além do QR/PIN.
- Keepalive TLS a cada 5 s; queda do TCP **não** derruba a sessão UDP de imediato (a T-304 decide).
- Mensagem `config` enviada quando o perfil muda (ex.: gyro ligado → client passa a enviar 0x02).
- Limites: máximo de 16 conexões pendentes; 5 s de timeout no handshake; mensagens de até 64 KB.

## Critérios de aceite
- [ ] Teste de integração: `padgaming-sim` faz o handshake completo e envia input.
- [ ] Um client com token revogado recebe `reject{revoked}`.
- [ ] Um `proto_ver` incompatível recebe `reject{version}` com a mensagem clara no client.

## Fora de escopo
- UI de pareamento (T-603).
