# T-904 — Pareamento QR/PIN + TLS pinning

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-903 |
| Status | ⬜ A fazer |

## Objetivo
Parear com o PC em segundos e com segurança.

## Escopo
- Scanner de QR (CameraX + ML Kit Barcode) lendo `vynpad://pair?...`; deep link também funciona (QR lido pela câmera nativa abre o app).
- Entrada de PIN (6 dígitos, teclado numérico) exibindo o fingerprint curto para conferência.
- Cliente TLS (OkHttp não se aplica, então `SSLSocket` com `TrustManager` customizado) fazendo **pinning** do fingerprint.
- Handshake `hello`/`welcome`/`reject` com mensagens de erro traduzidas.
- `device_id` (UUID gerado na instalação) e `device_token` guardados com EncryptedSharedPreferences/Keystore.
- Múltiplos servers pareados.

## Critérios de aceite
- [ ] Parear por QR e por PIN com o server real.
- [ ] Certificado diferente do fixado → conexão recusada com aviso de segurança.
- [ ] Token revogado → o app pede para parear de novo.

## Fora de escopo
- Envio de input (T-906).
