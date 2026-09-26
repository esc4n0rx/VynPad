# T-902 — Protocolo em Kotlin + test vectors

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-101, T-901 |
| Status | ⬜ A fazer |

## Objetivo
Implementação Kotlin do protocolo, binariamente idêntica à do Rust.

## Escopo
- Módulo `core:protocol` (Kotlin puro, JVM, testável sem emulador).
- Encode/decode com `ByteBuffer` little-endian reutilizável (sem alocação por pacote).
- ChaCha20-Poly1305: `javax.crypto` (API 28+) com fallback Tink/BouncyCastle para a API 26–27.
- Mensagens de controle com `kotlinx.serialization` + framing `u32 LE`.
- Testes com **os mesmos** `protocol/test-vectors/*.json` (lidos como recursos de teste).

## Critérios de aceite
- [ ] 100% dos test vectors passam.
- [ ] Encode + seal < 20 µs num aparelho intermediário (benchmark Jetpack Microbenchmark).

## Fora de escopo
- Sockets (T-906).
