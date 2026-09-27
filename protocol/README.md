# protocol

Artefatos do protocolo PadGaming v1 compartilhados entre o server (Rust) e o client Android (Kotlin). A especificação normativa está em [docs/00-visao-geral/PROTOCOLO.md](../docs/00-visao-geral/PROTOCOLO.md).

| Pasta           | Conteúdo                                                                                              |
| --------------- | ----------------------------------------------------------------------------------------------------- |
| `schemas/`      | JSON Schemas (draft 2020-12): mensagens de controle, resposta de descoberta e formato dos vectors     |
| `test-vectors/` | `udp.json`, `control.json`, `pairing-uri.json` — **gerados**, não edite à mão                         |
| `tools/`        | Implementação de referência (`reference.mjs`), definição dos vectors (`vectors.mjs`), gerador e check |

## Comandos

```sh
pnpm --filter @padgaming/protocol gen     # regrava test-vectors/*.json
pnpm --filter @padgaming/protocol check   # confere os vectors (roda no CI)
```

O `check` confirma que o ChaCha20-Poly1305 do Node bate com a RFC 8439, que os arquivos commitados são idênticos à saída do gerador, que eles validam nos schemas e que o decoder de referência produz exatamente o `expect` de cada vector.

## Usando os vectors

- **Rust (T-102):** para cada vector de `udp.json`, decodifique `packet_hex` com o contexto de `receiver` e compare com `expect`. Nos válidos sem `decode_only`, codifique `header` + `payload` e compare com `packet_hex`.
- **Kotlin (T-902):** os mesmos arquivos, lidos como recursos de teste.

Para mudar o protocolo, siga a política de versionamento (PROTOCOLO §10): atualize a spec, os schemas e `tools/vectors.mjs`, rode `gen` e registre a decisão em DECISOES.md.
