# T-903 — Descoberta NSD + broadcast + IP manual

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-902 |
| Status | ⬜ A fazer |

## Objetivo
O usuário abre o app e vê o PC na lista, sem digitar nada.

## Escopo
- `NsdManager` descobrindo `_padgaming._tcp` + fallback de broadcast `PGDISC1` (com `WifiManager.MulticastLock` enquanto busca).
- Anunciar `_padgaming-client._tcp` enquanto a tela Conectar está aberta (aparece em "Próximos" no server).
- Lista de servers com ícone do SO, nome, slots livres, "pareado" ou "novo".
- Adicionar manualmente por IP:porta.
- Servers pareados salvos com reconexão em um toque (e opção "conectar automaticamente ao abrir").
- Permissão `NEARBY_WIFI_DEVICES` (Android 13+) com explicação.

## Critérios de aceite
- [ ] O PC aparece em até 3 s na mesma rede.
- [ ] O celular aparece em "Próximos" no server.

## Fora de escopo
- Pareamento (T-904).
