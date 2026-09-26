# T-303 — Descoberta mDNS + broadcast

| Campo | Valor |
|---|---|
| Fase | F3 — Rede |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-102 |
| Status | ⬜ A fazer |

## Objetivo
Auto detect nos dois sentidos: o celular encontra o PC e o PC mostra os celulares próximos, sem digitar IP.

## Escopo
- **Anúncio** do server com `mdns-sd`: `_padgaming._tcp.local.` + TXT (ver PROTOCOLO §2); atualizar `free` quando os slots mudam.
- **Browse** de `_padgaming-client._tcp` → lista de "dispositivos próximos" (nome, modelo, IP, visto há X s, se já é pareado).
- **Fallback por broadcast**: listener UDP 47802 respondendo `PGDISC1`.
- Múltiplas interfaces: anunciar em todas as interfaces privadas ativas; ignorar interfaces virtuais (Hyper-V, WSL, VPN) por padrão, com opção de escolher a interface nas Configurações.
- Evento `devices:discovered` com debounce de 500 ms; expirar dispositivos sem anúncio há 15 s.
- Mostrar os IPs locais na UI (para conexão manual).

## Critérios de aceite
- [ ] O `padgaming-sim --discover` encontra o server via mDNS e via broadcast.
- [ ] Um client anunciado aparece na UI em até 3 s e some em até 15 s após sair.
- [ ] Com WSL/Hyper-V instalados, o anúncio sai pela interface Wi-Fi/Ethernet correta.

## Fora de escopo
- NSD no Android (T-903).
