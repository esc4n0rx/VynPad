# T-501 — Design system: tokens + componentes base

| Campo | Valor |
|---|---|
| Fase | F5 — UI Shell |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-002 |
| Status | ⬜ A fazer |

## Objetivo
Implementar a linguagem visual de [DESIGN-SYSTEM.md](../../00-visao-geral/DESIGN-SYSTEM.md) como tokens e componentes reutilizáveis.

## Escopo
- Tokens CSS (dark padrão + light) no `@theme` do Tailwind v4; `data-theme` no `<html>`.
- Accent customizável via CSS var (azul padrão #0A84FF).
- Componentes (shadcn/ui adaptado): Button, IconButton, Toggle (switch estilo macOS), SegmentedControl, Slider (com valor e marcação do padrão), Select, Input, Card de vidro, Dialog/Sheet com backdrop blur, Toast (sonner), Tooltip, Badge de status, Stepper, EmptyState, Skeleton, Kbd.
- Página interna `/dev/ui` (só em dev) mostrando todos os componentes em dark e light.
- Acessibilidade: foco visível com anel na cor do accent, contraste AA, navegação por teclado.

## Critérios de aceite
- [ ] Todos os componentes na página `/dev/ui`, nos dois temas.
- [ ] Nenhuma cor hardcoded fora dos tokens (regra de lint ou revisão).
- [ ] Animações respeitam `prefers-reduced-motion`.

## Fora de escopo
- Janela e vibrancy (T-502). Gamepad SVG (T-505).
