# PadGaming

Transforma um celular Android em um **controle Xbox 360 nativo** para PC. O client Android envia o input pela rede local; o server (Windows / Linux) cria um controle virtual que funciona em qualquer jogo XInput.

## Estrutura

| Pasta | Conteúdo |
|---|---|
| `apps/desktop` | Server desktop (Tauri 2 + React) |
| `apps/android` | Client Android (Kotlin + Compose) |
| `crates/` | Núcleo em Rust: protocolo, driver, mapeamento, rede, armazenamento, core, simulador |
| `protocol/` | Schemas, test vectors e implementação de referência do protocolo (compartilhados entre Rust e Kotlin) |
| `docs/` | Especificação, arquitetura, roadmap e tasks |

## Desenvolvimento

Requisitos: Rust (versão fixada em `rust-toolchain.toml`), Node ≥ 22 e pnpm.

```sh
cargo build --workspace
cargo test --workspace --exclude padgaming-desktop
cargo test -p padgaming-desktop   # separado: ver a nota no ci.yml (CRT estática do Tauri)
pnpm install
```

Comece pela [documentação](docs/README.md) e pelo [roadmap](docs/00-visao-geral/ROADMAP.md).
