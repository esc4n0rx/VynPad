# Contribuindo

O VynPad é um projeto proprietário (veja a [LICENSE](LICENSE)). Contribuições externas só são
aceitas com acordo prévio; combine antes de abrir um pull request. Este guia vale para quem já
trabalha no repositório.

## Ambiente

Rust (versão fixada em `rust-toolchain.toml`), Node 22 ou mais novo e pnpm. No Windows, o
[ViGEmBus](https://vigembusdriver.com/download/) só é necessário para os testes de hardware.

```sh
pnpm install            # também instala os hooks do git (lefthook)
pnpm --filter @vynpad/desktop tauri dev
```

## Como o trabalho é organizado

- Cada tarefa é um arquivo em [`docs/tasks/`](docs/tasks), com objetivo, escopo e critérios de aceite.
  O quadro está no [roadmap](docs/00-visao-geral/ROADMAP.md).
- **Uma task por vez.** Cada task fecha com um commit próprio e atualiza o status dela e do roadmap.
- Decisões técnicas e de produto vão para [`docs/00-visao-geral/DECISOES.md`](docs/00-visao-geral/DECISOES.md).
  Mudou o protocolo? Atualize o [PROTOCOLO](docs/00-visao-geral/PROTOCOLO.md) e regenere os vetores
  (`pnpm --filter @vynpad/protocol gen`).

## Commits

[Conventional Commits](https://www.conventionalcommits.org), em português, com a task no fim. O
`commitlint` roda no hook de commit.

```text
feat(desktop): auto-update com assinatura minisign (T-704)
fix(net): descarta pacote com sessão desconhecida antes da criptografia
docs(bench): registra a meta de RAM que falhou (T-702)
```

Tipos: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `ci`, `build`, `chore`. Um `!` depois do
escopo marca mudança incompatível. As notas de cada release são geradas destes commits.

## Antes de commitar

Rode o que o CI roda; o hook de commit já faz `rustfmt`, `prettier` e `eslint` nos arquivos alterados:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --exclude vynpad-desktop --locked
cargo test -p vynpad-desktop --locked
cargo deny check
pnpm format:check && pnpm lint && pnpm typecheck && pnpm test
pnpm --filter @vynpad/protocol check
```

Alguns lembretes:

- **Bindings tipados:** mudou comando, evento ou tipo em Rust? `UPDATE_BINDINGS=1 cargo test -p vynpad-desktop bindings_ts`.
- **Queries do sqlx:** mudou uma query? Rode `scripts/sqlx-prepare.sh` e versione o `.sqlx/`.
- **Migrations já publicadas nunca se editam**; crie uma nova.
- **Textos da interface** em PT-BR **e** EN (há um teste de paridade e um lint de texto fixo).
- **Hot path de input:** sem alocação, sem lock, fora do runtime async. Há testes que vigiam isso.
- **Segredos** (chaves, tokens, PINs) nunca vão para commits nem para logs (`Secret<T>`).

## Testes

| Camada | Como |
|---|---|
| Unidade e integração Rust | `cargo test`; o `vynpad-sim` faz o papel do celular |
| Propriedades e fuzz | `proptest` nos testes; `cargo fuzz` em `crates/vynpad-protocol/fuzz` (Linux) |
| Interface | Vitest (`pnpm test`) e E2E em [`e2e/`](e2e/README.md) |
| Cobertura | `cargo llvm-cov`; mínimo de 80% de linhas nos crates de domínio |
| Hardware | testes `#[ignore]` do ViGEmBus e do XInput |
| Manual | [docs/QA-CHECKLIST.md](docs/QA-CHECKLIST.md), antes de cada release |

## Pull requests

Use o modelo do repositório. Descreva o que muda e por quê, ligue a task e diga como testar.
O CI precisa estar verde: lint, dependências, Rust (Windows e Linux), cobertura, E2E e o build do app.

## Lançando uma versão

1. Atualize a versão em `apps/desktop/src-tauri/tauri.conf.json` (e nos `Cargo.toml`, se mudar).
2. Com o CI verde na `main`: `git tag vX.Y.Z && git push origin vX.Y.Z`.
3. O workflow de release compila, assina, gera o `latest.json` do auto-update e publica.
4. Execute e registre o [checklist de QA](docs/QA-CHECKLIST.md).
