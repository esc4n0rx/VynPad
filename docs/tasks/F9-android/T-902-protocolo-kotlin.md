# T-902 — Protocolo em Kotlin + test vectors

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-101, T-901 |
| Status | 🟨 Em andamento (subtask B: núcleo Rust e bindings feitos; falta o laço de envio, no T-906) |

> **Atualização (D-26, 2026-10-04):** o protocolo **não será reescrito em Kotlin**. O client usa o `vynpad-protocol` em Rust por UniFFI, então esta task vira "bindings UniFFI do núcleo + test vectors rodando nos dois lados": o Kotlin chama o mesmo código do server, e os `protocol/test-vectors/*.json` continuam como contrato. O escopo e os critérios abaixo ficam como referência do plano antigo e serão reescritos depois da prova de conceito do T-901.

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

## Andamento (2026-10-05)
Crate `crates/vynpad-client-core` criado, com `ClientSession`, leitura da URI de pareamento e da resposta de descoberta, e `CoreError` com os códigos estáveis. Testes em `tests/vectors.rs` contra `protocol/test-vectors/` (o primeiro input sai byte a byte igual ao vector `input_neutral`). O Gradle gera os bindings Kotlin e compila a `.so` para `arm64-v8a`, `armeabi-v7a` e `x86_64`; o app mostra as versões do núcleo em Ajustes → Sobre. Ver o README do crate.
