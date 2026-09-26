# T-003 — Lint, format, hooks e CI

| Campo | Valor |
|---|---|
| Fase | F0 — Fundação |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-002 |
| Status | ⬜ A fazer |

## Objetivo
Garantir qualidade constante desde o início, com feedback automático em cada push/PR.

## Escopo
- **Rust**: `rustfmt.toml`, `clippy` com `-D warnings`, `cargo-deny` (licenças + advisories), `cargo-machete` (deps não usadas).
- **Frontend**: ESLint (flat config, `typescript-eslint`, `react-hooks`, `jsx-a11y`), Prettier + plugin do Tailwind, `tsc --noEmit`.
- **Hooks**: `lefthook` rodando fmt + lint nos arquivos staged.
- **Commits**: Conventional Commits (`commitlint`), base para o changelog automático.
- **GitHub Actions**:
  - `ci.yml`: matriz `windows-latest` + `ubuntu-latest` → fmt, clippy, testes, lint, typecheck, build do Tauri (sem assinar).
  - Cache: `Swatinem/rust-cache` + cache do pnpm.
- Template de PR e de issue (bug/feature).

## Critérios de aceite
- [ ] Um PR com warning de clippy ou erro de lint falha no CI.
- [ ] O CI completo leva < 15 min com cache quente.
- [ ] `cargo deny check` passa.

## Fora de escopo
- Release e assinatura (T-703). Testes E2E (T-701).
