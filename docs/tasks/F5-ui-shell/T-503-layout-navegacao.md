# T-503 — Layout, sidebar, rotas e transições

| Campo | Valor |
|---|---|
| Fase | F5 — UI Shell |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-501, T-502 |
| Status | 🟨 Pronta; falta conferir no app (mouse, atalhos, sidebar recolhida) e o profiler dos 60 fps |

## Objetivo
Estrutura de navegação do app no padrão macOS (sidebar + conteúdo).

## Escopo
- Layout: barra de título (T-502) + sidebar translúcida de 220 px + área de conteúdo.
- Itens da sidebar: **Controles** (dashboard), **Dispositivos**, **Mapeamento**, **Latência**, separador, **Configurações**. No rodapé: status do server (● Online / driver / IP) e mini-indicadores P1–P4.
- Rotas com lazy loading; rota `/onboarding` em tela cheia, sem sidebar.
- Transições de página com Motion (fade + 8 px); indicador de seleção animado (`layoutId`).
- Atalhos: `Ctrl+1..5` para as seções, `Ctrl+,` para Configurações.
- Sidebar recolhível (só ícones) abaixo de 1000 px de largura.
- Error boundary por rota, com tela amigável e botão "Copiar detalhes".

## Critérios de aceite
- [ ] Navegação por mouse e teclado entre todas as seções.
- [ ] A sidebar recolhe automaticamente em janelas estreitas.
- [ ] Transições a 60 fps (verificado no profiler).

## Fora de escopo
- Conteúdo das telas (F6).
