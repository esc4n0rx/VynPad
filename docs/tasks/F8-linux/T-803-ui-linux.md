# T-803 — Adaptações de UI no Linux

| Campo | Valor |
|---|---|
| Fase | F8 — Linux |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-502, T-607 |
| Status | ⬜ A fazer |

## Objetivo
Manter a experiência macOS-like no Linux, com degradação elegante.

## Escopo
- Transparência: WebKitGTK + compositor; testar GNOME (Wayland), KDE Plasma (Wayland/X11); fallback para sólido quando a transparência não é suportada ou fica com artefatos.
- Cantos arredondados via CSS + região transparente; sombra CSS.
- Redimensionamento pelas bordas em janela sem decoração (verificar o suporte do Tauri no Wayland).
- Tray: `libayatana-appindicator`; no GNOME, orientar sobre a extensão AppIndicator e manter o app funcional sem tray.
- Autostart: arquivo `.desktop` em `~/.config/autostart`.
- Notificações via `libnotify`.
- Fonte e escala HiDPI (fractional scaling).

## Critérios de aceite
- [ ] Checklist visual aprovado em GNOME e KDE.
- [ ] Sem tray disponível, fechar a janela oferece "Sair" em vez de sumir.

## Fora de escopo
- Temas GTK nativos.
