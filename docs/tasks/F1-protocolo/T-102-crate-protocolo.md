# T-102 — Crate `vynpad-protocol`

| Campo | Valor |
|---|---|
| Fase | F1 — Protocolo |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-001, T-101 |
| Status | ✅ Concluída |

## Objetivo
Implementar em Rust a (de)serialização e a criptografia dos pacotes, sem alocação e com testes contra os test vectors.

## Escopo
- Tipos: `PacketHeader`, `InputState`, `MotionState`, `Ping`, `Pong`, `Rumble`, `Buttons` (bitflags), `ProtoError` com os códigos da PROTOCOLO §6.5.
- `encode_into(&mut [u8]) -> usize` e `decode(&[u8]) -> Result<Packet, ProtoError>` sem alocação.
- `SessionCipher` com ChaCha20-Poly1305 (crate `chacha20poly1305`), seal/open in-place.
- `ReplayWindow` (último `seq` aceito; **sem** wrap-around, PROTOCOLO §6.3) e contador de envio que sinaliza o esgotamento do `seq`.
- Mensagens de controle (`Hello`, `Welcome`, `Reject`, `Config`, `Bye`) com `serde` + framing `u32 LE`.
- Feature `std` / `no_std`-friendly no núcleo (facilita um futuro port).

## Detalhes técnicos
- Parsing manual com `u16::from_le_bytes` etc. (ou `zerocopy`), sem `unsafe`.
- Benchmark `criterion`: decode + open < 2 µs por pacote.
- Fuzzing com `cargo-fuzz` no `decode` (target em `fuzz/`).

## Critérios de aceite
- [x] 100% dos test vectors passam (`udp.json` e `control.json`; os erros batem com os códigos esperados).
- [x] Fuzzer roda 10 min sem pânico — ver notas: o libFuzzer roda no workflow **Fuzz** (Linux); localmente, 10 min do estresse por mutação.
- [x] Benchmark dentro da meta.
- [x] Documentação `rustdoc` em todos os itens públicos (`#![warn(missing_docs)]` + clippy `-D warnings`).

## Fora de escopo
- Sockets e rede (F3). TLS (T-103).

## Notas de implementação
- **API UDP:**
  - `parse_header(buf, role)` faz os passos 1–6 da §6.5.
  - O server procura a sessão pelo `session_id` (passo 7) e chama `Session::receive_parsed` ou `open_packet`, que fazem os passos 8–10.
  - O client (e o simulador) usam `Session::receive`/`send`, que cuidam do `seq` com `SeqCounter`.
  - `open_packet` decifra uma cópia do payload na pilha, então o datagrama não é alterado e nada é alocado.
  - `ReplayWindow` só avança depois da última validação.
- **`seal_packet`** recebe o `seq` explicitamente; é o que os test vectors usam. `Session::send` não consome `seq` se o buffer for pequeno ou o payload for da direção errada.
- **Canal de controle:**
  - `decode_body`/`decode_frame` → `Incoming::{Message, Ignored}`. Um `type` desconhecido é ignorado, como manda a §5.2.
  - As faixas e tamanhos que o serde não cobre ficam em `ControlMessage::validate`, chamado também por `encode_frame`.
  - Motivos desconhecidos de `reject`/`bye` viram `Unknown`.
  - Tokens, chave e PIN ficam em `Secret` e não aparecem no `Debug`.
- **`no_std`:** o núcleo compila com `--no-default-features`; a feature `std` (padrão) traz o `control` com serde/serde_json/base64.
- **Dependências novas:** `chacha20poly1305` 0.11 (API `AeadInOut` do `aead` 0.6), `bitflags` 2, `base64` 0.22 (mesma versão do Tauri); em dev, `criterion` e `hex`.
- **Benchmark** (release, máquina de desenvolvimento): decode + open ≈ **1,13 µs**; encode + seal ≈ 1,12 µs; com motion, o custo é o mesmo. O custo é dominado pelo overhead fixo do AEAD em mensagens pequenas.
- **Fuzzing:**
  - O `cargo-fuzz` não linka no Windows/MSVC sem a runtime do ASan do Visual Studio (com `-s none`, o erro é `__start___sancov_cntrs`), e não há WSL nesta máquina.
  - Por isso os três alvos em `fuzz/` rodam no workflow `.github/workflows/fuzz.yml`, no Linux: 10 min cada, disparo manual e semanal.
  - Localmente, `tests/stress.rs` faz fuzzing por mutação na toolchain estável, partindo dos vectors. O oráculo exige que todo pacote aceito seja idêntico ao que o encoder produz para o conteúdo decodificado.
  - Rodada local de 10 min (release, semente `0x135283f`): 568,5 milhões de iterações, sem pânico nem pacote adulterado aceito.
- **Robustez:** `open_packet` devolve `BadLength` se receber um cabeçalho que não é do datagrama passado, em vez de entrar em pânico ao fatiar.
