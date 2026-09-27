# T-002 — Scaffold Tauri 2 + React/TS/Vite/Tailwind

| Campo | Valor |
|---|---|
| Fase | F0 — Fundação |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-001 |
| Status | ✅ Concluída (2026-09-26) |

## Objetivo
App desktop abrindo uma janela com React renderizado, pronto para receber o design system e a integração com o core.

## Escopo
- `apps/desktop` criado com `create-tauri-app` (template React + TS + Vite), ajustado ao monorepo; adicionar `apps/desktop/src-tauri` aos `members` do `Cargo.toml` raiz.
- Tailwind CSS v4 via plugin `@tailwindcss/vite` (dispensa PostCSS); alias `@/` → `src/`.
- Dependências base: `react-router` (ou TanStack Router), `zustand`, `@tanstack/react-query`, `motion`, `lucide-react`, `clsx`, `tailwind-merge`, `class-variance-authority`, `@radix-ui/*` via shadcn/ui CLI.
- Fonte Inter empacotada localmente (sem CDN; o app funciona offline).
- `tauri.conf.json`: identificador `com.padgaming.server`, nome de produto "PadGaming", janela 1100×720 (mínimo 900×600), `decorations: false`, `transparent: true` (o visual em si fica na T-502).
- Capabilities do Tauri 2 (`capabilities/default.json`) com o mínimo necessário.
- Plugins iniciais: `tauri-plugin-single-instance` (reforça a janela existente), `tauri-plugin-log` *(removido na T-004, ver D-19)*.
- Scripts: `pnpm dev`, `pnpm build`, `pnpm tauri dev`, `pnpm tauri build`.

## Detalhes técnicos
- `src-tauri` depende de `padgaming-core` (ainda vazio) para já validar a ligação entre os crates.
- CSP restritiva em `tauri.conf.json`.

## Critérios de aceite
- [x] `pnpm tauri dev` abre a janela com hot reload funcionando.
- [x] `pnpm tauri build` gera o executável no Windows.
- [x] Uma segunda instância só foca a primeira.

## Fora de escopo
- Vibrancy, cantos e semáforo (T-502). Design system (T-501).

## Notas de implementação
- Versões resolvidas: Tauri 2, React 19, Vite 8, TypeScript 6, Tailwind 4.3, react-router 8, TanStack Query 5, Motion 13.
- Roteador em **hash** (`createHashRouter`) para não depender de fallback de rotas no protocolo de assets do Tauri.
- Barra de título **provisória** (`src/components/TitleBar.tsx`) para a janela sem moldura ser arrastável e fechável; o visual final é da T-502.
- shadcn/ui: só o `components.json` e o `cn()`. Os componentes entram via CLI na T-501.
- Command `app_info` valida a ponte React → Rust → `padgaming-core`.
- `pnpm tauri build` gera o instalador NSIS e o MSI em `target/release/bundle/`.
