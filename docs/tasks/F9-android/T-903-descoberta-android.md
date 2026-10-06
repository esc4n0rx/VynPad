# T-903 — Descoberta NSD + broadcast + IP manual

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-902 |
| Status | 🟨 Em andamento (subtask C: descoberta, radar e IP manual feitos; faltam permissões Android 17 e servidores salvos) |

## Objetivo
O usuário abre o app e vê o PC na lista, sem digitar nada.

## Escopo
- `NsdManager` descobrindo `_vynpad._tcp` + fallback de broadcast `VPDISC1` (com `WifiManager.MulticastLock` enquanto busca).
- Anunciar `_vynpad-client._tcp` enquanto a tela Conectar está aberta (aparece em "Próximos" no server).
- Lista de servers com ícone do SO, nome, slots livres, "pareado" ou "novo".
- Adicionar manualmente por IP:porta.
- Servers pareados salvos com reconexão em um toque (e opção "conectar automaticamente ao abrir").
- Permissão `NEARBY_WIFI_DEVICES` (Android 13+) com explicação.

## Critérios de aceite
- [ ] O PC aparece em até 3 s na mesma rede.
- [ ] O celular aparece em "Próximos" no server.

## Fora de escopo
- Pareamento (T-904).

## Requisito de UX (2026-10-05)
A busca de PCs na tela Conectar é um **radar animado no tema do app**: varredura circular em accent, anéis que expandem e cada PC descoberto surge como um ponto que "acende" (glow e haptic de confirmação). Tocar no ponto abre a configuração/conexão daquele PC (nome, IP, parear). Reaproveita o `Radar` do `ConnectScreen`.

## Andamento (2026-10-05)
- **Broadcast `VPDISC1` em Rust** (`BroadcastScanner` no `vynpad-client-core`): reenvia o pedido a cada 1 s, entrega cada server por callback (`ScanListener`) com o IP de origem e deduplica por server e origem; testes com um server em loopback.
- **mDNS pelo `NsdManager`** (`NsdSupport`): descobre `_vynpad._tcp` e anuncia `_vynpad-client._tcp` com `id`, `name` e `model`. Confirmado em aparelho (Android 15): o `NsdManager` **aceita** o tipo com mais de 15 caracteres e o celular aparece em "Próximos" no server.
- **Motor** (`DiscoveryEngine`): mescla os dois canais por `id`, com `MulticastLock` enquanto busca; busca de 8 s, o PC aparece em menos de 3 s.
- **UI:** radar animado com varredura e pontos tocáveis, lista de PCs, folha de detalhes (sistema, endereço, slots, fingerprint, origem) e "Adicionar por IP" com validação. O botão "Parear" fica desabilitado até a subtask D.
- **Abertura animada** (`SplashScreen`) e splash do sistema no Android 12+.
- **Pendente:** servidores pareados salvos e reconexão em um toque (depende da D) e a permissão de rede local (`ACCESS_LOCAL_NETWORK`) quando o `targetSdk` subir para 37.
- **Atenção no teste:** o `padgaming-desktop.exe` antigo (pré-renomeação) responde só ao broadcast antigo; teste com o build atual do VynPad ou com `cargo run -p vynpad-net --example discovery_server`.
