# T-802 — Permissões udev + onboarding Linux

| Campo | Valor |
|---|---|
| Fase | F8 — Linux |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-801, T-601 |
| Status | ⬜ A fazer |

## Objetivo
Rodar sem root, com um passo de configuração guiado.

## Escopo
- Regra udev `60-padgaming.rules`: `KERNEL=="uinput", SUBSYSTEM=="misc", TAG+="uaccess", OPTIONS+="static_node=uinput"` (+ carregar o módulo `uinput` via `/etc/modules-load.d/padgaming.conf`).
- Instalação pelos pacotes deb/rpm (T-804); no AppImage, o passo do onboarding executa com `pkexec`.
- O passo "Driver" do onboarding vira "Permissões" no Linux: detecta, explica e corrige.
- Firewall: detectar `ufw`/`firewalld` ativos e oferecer o comando (ou executar via `pkexec`) para abrir as portas.
- mDNS: coexistência com o `avahi-daemon` (validar que o `mdns-sd` funciona junto; senão, usar o Avahi via D-Bus).

## Critérios de aceite
- [ ] Ubuntu 24.04, Fedora 40+ e Arch/SteamOS (desktop mode): do zero até funcionando pelo onboarding, sem terminal.

## Fora de escopo
- Suporte a distros imutáveis além do SteamOS (documentar a limitação).
