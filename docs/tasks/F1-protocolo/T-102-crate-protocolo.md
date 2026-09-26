# T-102 — Crate `padgaming-protocol`

| Campo | Valor |
|---|---|
| Fase | F1 — Protocolo |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-001, T-101 |
| Status | ⬜ A fazer |

## Objetivo
Implementar em Rust a (de)serialização e a criptografia dos pacotes, sem alocação e com testes contra os test vectors.

## Escopo
- Tipos: `PacketHeader`, `InputState`, `MotionState`, `Ping`, `Pong`, `Rumble`, `Buttons` (bitflags).
- `encode_into(&mut [u8]) -> usize` e `decode(&[u8]) -> Result<Packet, ProtoError>` sem alocação.
- `SessionCipher` com ChaCha20-Poly1305 (crate `chacha20poly1305`), seal/open in-place.
- `ReplayWindow` (último `seq` aceito, com wrap-around).
- Mensagens de controle (`Hello`, `Welcome`, `Reject`, `Config`, `Bye`) com `serde` + framing `u32 LE`.
- Feature `std` / `no_std`-friendly no núcleo (facilita um futuro port).

## Detalhes técnicos
- Parsing manual com `u16::from_le_bytes` etc. (ou `zerocopy`), sem `unsafe`.
- Benchmark `criterion`: decode + open < 2 µs por pacote.
- Fuzzing com `cargo-fuzz` no `decode` (target em `fuzz/`).

## Critérios de aceite
- [ ] 100% dos test vectors passam.
- [ ] Fuzzer roda 10 min sem pânico.
- [ ] Benchmark dentro da meta.
- [ ] Documentação `rustdoc` em todos os itens públicos.

## Fora de escopo
- Sockets e rede (F3). TLS (T-103).
