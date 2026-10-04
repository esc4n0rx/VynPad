# T-103 — TLS, certificado e pareamento QR/PIN

| Campo | Valor |
|---|---|
| Fase | F1 — Protocolo |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-102 |
| Status | ✅ Concluída |

## Objetivo
Garantir que só dispositivos autorizados controlem o PC, sem exigir que o usuário entenda de certificados.

## Escopo
- **Identidade do server**: no primeiro boot, gerar par de chaves + certificado autoassinado (`rcgen`, ECDSA P-256, validade de 10 anos) e o `server_id` (UUID). Guardar no diretório de dados com permissões restritas.
- **Fingerprint**: SHA-256 do certificado (DER), base64url.
- **Pairing tokens**: 128 bits, uso único, TTL de 5 min, guardados só em memória.
- **URI do QR** (`vynpad://pair?...`) e geração do QR (crate `qrcode` → SVG enviado à UI).
- **PIN**: 6 dígitos, TTL de 2 min, rate limit (5 tentativas → bloqueio de 60 s por IP).
- **Device token**: 256 bits emitidos ao parear; persiste o hash SHA-256 (via trait `DeviceStore`, implementada na T-403).
- Revogação: um token revogado resulta em `reject{revoked}`.
- Comparação em tempo constante (`subtle`).

## Detalhes técnicos
- Servidor TLS com `rustls` + `tokio-rustls` (TLS 1.3 apenas).
- Trait `PairingService` isolada da rede para testes unitários.
- Opção "Regenerar identidade do server" (invalida todos os pareamentos) documentada para a T-605.

## Critérios de aceite
- [x] Um token expirado ou reutilizado é rejeitado.
- [x] O 6º PIN errado dispara o bloqueio.
- [x] O banco nunca contém tokens em texto puro.
- [x] Testes unitários cobrindo cada caminho de `auth`.

## Fora de escopo
- Loop de accept TCP e sessões (T-302). Telas de pareamento (T-603).

## Implementação
- `vynpad-net::identity`: `ServerIdentity` (`load_or_create`, `regenerate`, `tls_config` só TLS 1.3, provedor `ring`).
- `vynpad-net::pairing`: `PairingService`/`PairingManager`, `DeviceStore`, `Clock` injetável.
- `vynpad-net::qr`: SVG do QR. `vynpad-protocol::pairing_uri`: encoder/parser da URI, conferido contra `pairing-uri.json`.
- O PIN vale 2 min (PROTOCOLO §4.3 atualizado). O bloqueio conta erros por IP **e** globalmente.
