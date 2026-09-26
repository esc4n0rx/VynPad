# T-502 — Janela custom: vidro, cantos, semáforo

| Campo | Valor |
|---|---|
| Fase | F5 — UI Shell |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-002 |
| Status | ⬜ A fazer |

## Objetivo
A janela deve parecer um app nativo do macOS Sonoma rodando no Windows.

## Escopo
- Tauri: `decorations: false`, `transparent: true`, `shadow: true`.
- Vibrancy com o crate `window-vibrancy`:
  - Windows 11 → `apply_mica(dark = true)` (ou `apply_tabbed` se ficar melhor).
  - Windows 10 → `apply_acrylic` com tint escuro.
  - Detectar a versão do Windows e cair para fundo sólido quando a transparência estiver desativada no sistema.
- Cantos: nativos no Win11 (DWM); no Win10, container com `border-radius: 12px` e sombra CSS.
- **Barra de título** React (44 px) com `data-tauri-drag-region`, título central opcional e **semáforo** à esquerda (fechar / minimizar / maximizar), com ícones no hover e estado inativo acinzentado quando a janela perde o foco.
- Duplo clique na barra = maximizar/restaurar. Suporte a Snap Layouts no Win11 (hover no botão maximizar — avaliar a viabilidade; senão, documentar).
- Fechar → minimiza para a bandeja se a opção estiver ativa (T-607).
- Lembrar tamanho e posição (`tauri-plugin-window-state`).

## Critérios de aceite
- [ ] Win11: Mica visível, cantos arredondados, sombra nativa.
- [ ] Win10: acrylic ou fallback sólido sem artefatos nas bordas.
- [ ] Arrastar, redimensionar pelas bordas, minimizar, maximizar e fechar funcionam.

## Fora de escopo
- Adaptações de Linux (T-803).
