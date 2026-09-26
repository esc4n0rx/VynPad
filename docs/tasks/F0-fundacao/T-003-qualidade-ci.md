# T-003 — Lint, format, hooks e CI

| Campo | Valor |
|---|---|
| Fase | F0 — Fundação |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-002 |
| Status | ✅ Concluída (2026-09-26) — falta validar no GitHub Actions |

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
- [x] Um PR com warning de clippy ou erro de lint falha no CI. *(validado localmente: `clippy -D warnings` sai com erro; o workflow roda o mesmo comando)*
- [ ] O CI completo leva < 15 min com cache quente. *(pendente: medir no primeiro push)*
- [x] `cargo deny check` passa.

## Fora de escopo
- Release e assinatura (T-703). Testes E2E (T-701).

## Notas de implementação
- **Hooks** (`lefthook.yml`): pre-commit formata e corrige os arquivos staged (rustfmt, Prettier, ESLint); commit-msg roda o commitlint; pre-push roda o clippy. São instalados pelo script `prepare` da raiz (`pnpm install`); o postinstall do pacote `lefthook` fica bloqueado em `pnpm-workspace.yaml`.
- **ESLint**: `recommendedTypeChecked` do typescript-eslint (regras que usam os tipos), react-hooks, react-refresh e jsx-a11y.
- **cargo-deny**: licenças permissivas liberadas; os crates do workspace ficam fora da checagem de licença até a licença do projeto ser definida. O aviso `RUSTSEC-2024-0370` (`proc-macro-error` sem manutenção) foi ignorado com justificativa, pois chega só pelo gtk-rs do Tauri no Linux.
- **cargo-machete** apontou dependências declaradas sem uso (entre crates internos, desde a T-001). Foram removidas; cada task adiciona a dependência quando passar a usá-la.
- **CI** (`.github/workflows/ci.yml`): jobs `lint`, `deps`, `rust` (Windows + Ubuntu 22.04) e `bundle` (instaladores sem assinatura, publicados como artefato por 7 dias). O job `rust` compila o frontend antes do clippy, porque o `generate_context!` do Tauri exige o `dist/`.
