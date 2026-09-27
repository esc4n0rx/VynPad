# T-101 — Especificação final do protocolo v1 + test vectors

| Campo | Valor |
|---|---|
| Fase | F1 — Protocolo |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | — |
| Status | ✅ Concluída |

## Objetivo
Congelar o contrato binário entre client e server para que o server (Rust) e o client Android (Kotlin) sejam desenvolvidos de forma independente e compatível.

## Escopo
- Revisar e finalizar [PROTOCOLO.md](../../00-visao-geral/PROTOCOLO.md):
  - Layout de todos os pacotes UDP (0x01, 0x02, 0x10, 0x11, 0x20).
  - Mensagens JSON do canal de controle (com JSON Schema em `protocol/schemas/`).
  - Construção do nonce, direção e tratamento de wrap-around de `seq`.
  - Regras de timeout, taxa de envio e códigos de `reject`.
  - Política de versionamento (`proto_ver` e o que conta como mudança compatível).
- Definir o formato e gerar os **test vectors** em `protocol/test-vectors/`:
  - Encode/decode de cada tipo de pacote, válido e inválido (magic errado, tag adulterada, tamanho curto, seq antigo).
  - Exemplos de mensagens de controle.
- Documentar o mapeamento dos bits de botão para os nomes XInput.

## Critérios de aceite
- [x] PROTOCOLO.md marcado como "v1 congelado".
- [x] ≥ 15 test vectors cobrindo casos válidos e inválidos (79: 32 UDP, 31 de controle, 16 de URI).
- [x] Revisão feita pensando na implementação em Kotlin (sem tipos unsigned que atrapalhem, endianness explícita).

## Fora de escopo
- Implementação (T-102, T-902).

## Notas de implementação
- **Correções no rascunho:**
  - O nonce usava `seq` u64, mas o cabeçalho tem `seq` u32. Agora o nonce é `direction (u32 LE) ‖ seq (u32 LE) ‖ 0`.
  - O `seq` **não dá a volta**: ao esgotar, abre-se uma sessão nova com chave nova, porque o wrap-around repetiria o nonce (D-20).
  - `rumble_ack_seq` (u16) não conseguia confirmar retransmissões, que usam um `seq` novo cada. Agora o Rumble carrega um `rumble_id` (u16) por mudança de estado, e o InputState devolve `rumble_ack_id`. O payload do Rumble passou a ter 4 bytes (`flags` saiu).
- **Kotlin:** não existe u64. Os timestamps são i64 ≥ 0 e o `session_id` fica em 1–2³¹−1, cabendo em `Int` também no JSON. A §12 da spec traz as receitas de leitura sem sinal.
- **Adições:**
  - Ordem normativa de validação com códigos de erro (§6.5).
  - Decoders mascaram os bits reservados.
  - `auth` com discriminador `method`.
  - Objeto `config` completo no `welcome`.
  - URI com `v=1` e vários IPs em `h`, porque o PC costuma ter adaptadores virtuais.
  - Resposta de broadcast com `tcp`.
  - Referencial do gyro e ping UDP a 4 Hz.
- **Timeouts:** a queda do TLS encerra a sessão e reserva o slot por 30 s. O silêncio do UDP só leva ao estado neutro (250 ms) e a "suspensa" (5 s). T-302 e T-304 foram alinhadas a isso.
- **Vectors:**
  - São gerados por `protocol/tools/gen.mjs`, uma implementação de referência em Node com o ChaCha20-Poly1305 do OpenSSL (D-21).
  - `pnpm --filter @padgaming/protocol check` roda no job de lint do CI.
  - Conferência cruzada manual: os 11 pacotes UDP válidos abrem com a crate `chacha20poly1305` (RustCrypto) e batem com `payload_plain_hex`, e os 5 `bad_tag` são rejeitados.
- `protocol/` virou pacote do workspace pnpm (`@padgaming/protocol`, com `ajv` e `prettier`), com job de prettier próprio no lefthook.
