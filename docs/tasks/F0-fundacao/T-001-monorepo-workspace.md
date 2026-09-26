# T-001 — Monorepo e workspace Cargo

| Campo | Valor |
|---|---|
| Fase | F0 — Fundação |
| Prioridade | P0 |
| Estimativa | P |
| Depende de | — |
| Status | ✅ Concluída (2026-09-26) |

## Objetivo
Criar a estrutura do repositório descrita em [ARQUITETURA.md](../../00-visao-geral/ARQUITETURA.md) para que todas as outras tasks tenham onde viver.

## Escopo
- `git init`, `.gitignore` (Rust, Node, Tauri, Android, `.env`), `.gitattributes` (LF).
- `Cargo.toml` raiz com `[workspace]` e `resolver = "2"`, incluindo `crates/*` (o `apps/desktop/src-tauri` entra na T-002, já que o Cargo não aceita membro inexistente).
- Crates vazios com `lib.rs` e README de uma linha: `padgaming-protocol`, `padgaming-driver`, `padgaming-mapping`, `padgaming-net`, `padgaming-storage`, `padgaming-core`, `padgaming-sim` (bin).
- `[workspace.dependencies]` centralizando versões (tokio, serde, thiserror, tracing, anyhow).
- `[workspace.lints]` com `clippy::pedantic` em warn (exceto regras ruidosas) e `unsafe_code = "deny"` (liberado só no driver).
- `rust-toolchain.toml` fixando o stable atual + componentes `clippy`, `rustfmt`.
- `pnpm-workspace.yaml` com `apps/desktop`.
- `README.md` raiz com visão rápida e link para `docs/`.
- `LICENSE`: **pendente de decisão do dono do projeto** (sugestão: MIT/Apache-2.0 dual).

## Detalhes técnicos
- Perfil `release`: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` apenas no binário (não nos testes), `strip = true`.
- Perfil `dev` com `opt-level = 1` para `padgaming-mapping` e `padgaming-protocol` (hot path mais realista em debug).

## Critérios de aceite
- [x] `cargo build --workspace` e `cargo test --workspace` passam num checkout limpo.
- [x] `pnpm install` na raiz funciona.
- [x] Árvore de pastas igual à documentada.

## Fora de escopo
- Código do app Tauri (T-002). CI (T-003).
