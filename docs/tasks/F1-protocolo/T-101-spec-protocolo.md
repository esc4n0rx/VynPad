# T-101 — Especificação final do protocolo v1 + test vectors

| Campo | Valor |
|---|---|
| Fase | F1 — Protocolo |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | — |
| Status | ⬜ A fazer |

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
- [ ] PROTOCOLO.md marcado como "v1 congelado".
- [ ] ≥ 15 test vectors cobrindo casos válidos e inválidos.
- [ ] Revisão feita pensando na implementação em Kotlin (sem tipos unsigned que atrapalhem, endianness explícita).

## Fora de escopo
- Implementação (T-102, T-902).
