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
- Keepalive conforme PROTOCOLO §5.3 (`ping` após 5 s sem enviar; fecha após 15 s sem receber). A queda do TLS encerra a sessão UDP (a chave é descartada) e o slot fica reservado por 30 s (PROTOCOLO §9, T-304).
- Mensagem `config` enviada quando o perfil muda (ex.: gyro ligado → client passa a enviar 0x02).
- Limites: máximo de 16 conexões pendentes; `hello` em até 10 s; frames de até 16 KiB (PROTOCOLO §5.1).

## Critérios de aceite
- [ ] Teste de integração: `padgaming-sim` faz o handshake completo e envia input.
- [ ] Um client com token revogado recebe `reject{revoked}`.
- [ ] Um `proto_ver` incompatível recebe `reject{version}` com a mensagem clara no client.

## Fora de escopo
- UI de pareamento (T-603).
