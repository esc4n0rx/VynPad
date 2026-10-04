# vynpad-protocol

Implementação em Rust do protocolo VynPad v1 ([PROTOCOLO.md](../../docs/00-visao-geral/PROTOCOLO.md)).

| Módulo | Conteúdo |
|---|---|
| `packet` | Layout dos pacotes UDP, `parse_header` (passos 1–6 da §6.5) |
| `crypto` | `SessionCipher` (ChaCha20-Poly1305), `seal_packet`, `open_packet` (passos 8–10) |
| `session` | `ReplayWindow` (anti-replay sem wrap-around), `SeqCounter`, `Session` |
| `buttons` | `Buttons` (bitmask XInput) |
| `control` | Mensagens JSON do canal de controle e framing `u32 LE` (feature `std`) |
| `secret` | `Secret<T>`, que nunca aparece em logs |

O núcleo UDP é `no_std` (`--no-default-features`), não aloca e não usa `unsafe`.

## Testes

```sh
cargo test -p vynpad-protocol                    # unitários + test vectors + estresse curto
VYNPAD_STRESS_SECS=600 cargo test -p vynpad-protocol --release --test stress -- --nocapture
cargo bench -p vynpad-protocol                   # meta: decode + open < 2 µs
```

- `tests/vectors.rs` roda **todos** os vectors de `protocol/test-vectors/udp.json` e `control.json`. Nos casos válidos, o pacote é recodificado a partir do `expect` e comparado byte a byte.
- `tests/stress.rs` muta os vectors aleatoriamente e confere que nada entra em pânico e que nenhum pacote adulterado é aceito.

## Fuzzing

Os alvos ficam em `fuzz/` (`udp_decode`, `udp_roundtrip`, `control_frame`) e exigem nightly e Linux: no Windows/MSVC, o libFuzzer precisa da runtime do ASan do Visual Studio.

```sh
cd crates/vynpad-protocol/fuzz
cargo +nightly fuzz run udp_decode -- -max_total_time=600
```

No CI, o workflow **Fuzz** roda os três alvos por 10 min cada (manual e semanal).
