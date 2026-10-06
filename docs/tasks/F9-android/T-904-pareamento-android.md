# T-904 — Pareamento QR/PIN + TLS pinning

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-903 |
| Status | 🟨 Em andamento (subtask D: implementado e testado contra o server em loopback; falta validar no aparelho com o desktop real) |

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

## Andamento (2026-10-05)
- **Canal de controle no núcleo Rust** (`ControlConnection` em `vynpad-client-core`): TLS 1.3 com `rustls`, confiança por `TrustPolicy` (`Pinned` com o fingerprint inteiro do QR ou do pareamento, `ShortFingerprint` com os 8 bytes da descoberta para o PIN, `FirstUse`), `hello`/`welcome`/`reject`. O certificado errado é recusado **antes** de qualquer credencial ser enviada (`FingerprintMismatch`). Em vez de `SSLSocket` com `TrustManager` (plano antigo), o pinning vive no Rust, igual para Android e iOS.
- **Testes contra o server de controle de verdade** (`tests/control.rs`, loopback): pareamento por QR e por PIN, reconexão com `device_token` (mesmo slot, sem novo token), PIN errado (`bad_auth`), bloqueio após 5 erros (`locked` com `retry_after_s`), certificado diferente.
- **Android:** `device_token` e dados dos PCs pareados em `SecureBlobStore` (AES-256-GCM, chave no Android Keystore) em vez de `EncryptedSharedPreferences` (obsoleta); câmera com CameraX + ML Kit (modelo embutido); tela de pareamento com seletor Câmera/PIN, teclado numérico próprio, células de PIN que tremem no erro, "Conectando" com anéis, selo de sucesso animado e mensagens de erro traduzidas; deep link `vynpad://pair` (QR lido pela câmera nativa); vários PCs pareados; "Esquecer PC".
- **Sessão depois do pareamento (correção):** a primeira versão encerrava a sessão logo após receber o token, e o PC mostrava o celular como desconectado. Agora o pareamento **entrega a mesma conexão** ao `AndroidSessionManager`, e o núcleo Rust mantém o keepalive (`ControlConnection::start`): responde aos `ping` do server na hora, manda o seu a cada 5 s de silêncio e dá a conexão por morta em 15 s. O app reconecta sozinho com o `device_token` e o backoff do protocolo (0,5 → 1 → 2 → 4 → 5 s, desiste em 2 min), trata `bye:kicked` e `bye:revoked` (este remove o pareamento, pedindo para parear de novo) e mostra o estado numa faixa no topo da tela Conectar. Testes contra o server real provam o conserto: com o keepalive a sessão sobrevive ao limite de silêncio do server; sem ele, o server a derruba.
- **Validado no aparelho (2026-10-05):** pareamento por QR com o desktop real; recusa de PIN errado pelo desktop real.
- **Pendente:** validar de novo no aparelho a conexão mantida depois do pareamento e o PIN correto; o job de CI do Android.
