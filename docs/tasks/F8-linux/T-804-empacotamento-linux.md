# T-804 — Empacotamento AppImage/deb/rpm

| Campo | Valor |
|---|---|
| Fase | F8 — Linux |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-801 |
| Status | ⬜ A fazer |

## Objetivo
Distribuir para as principais famílias de distros.

## Escopo
- Bundles do Tauri: **AppImage** (com auto-update), **.deb**, **.rpm**.
- deb/rpm instalam a regra udev e o `modules-load` (T-802) via scripts de pós-instalação, e removem na desinstalação.
- Adicionar o Linux ao `release.yml` (build em `ubuntu-22.04` para compatibilidade de glibc).
- Flatpak: avaliar a viabilidade (acesso ao `/dev/uinput` exige `--device=all`); documentar a decisão.
- Assinatura dos pacotes (GPG) e checksums.

## Critérios de aceite
- [ ] Os três formatos instalam e funcionam nas distros da T-802.
- [ ] O AppImage se atualiza sozinho.

## Fora de escopo
- AUR (a comunidade pode manter).
