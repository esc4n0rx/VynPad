# T-303 — Descoberta mDNS + broadcast

| Campo | Valor |
|---|---|
| Fase | F3 — Rede |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-102 |
| Status | 🟨 Pronta; falta validar com WSL/Hyper-V instalados e a UI (T-603) |

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
- [x] O `padgaming-sim discover` encontra o server via mDNS e via broadcast.
- [x] Um client anunciado aparece na UI em até 3 s e some em até 15 s após sair.
- [ ] Com WSL/Hyper-V instalados, o anúncio sai pela interface Wi-Fi/Ethernet correta. (Coberto por teste unitário com um PC "cheio" de interfaces virtuais; falta conferir numa máquina com WSL/Hyper-V ativos: a de desenvolvimento só tem o Wi-Fi.)

## Fora de escopo
- NSD no Android (T-903).

## Situação

Pronto, em `padgaming_net::discovery` (`Discovery`) e `padgaming_protocol::discovery` (tipos compartilhados):
- Anúncio `_padgaming._tcp` com o TXT do §3.1 (`Announcer`); o `free` acompanha os slots (`Discovery::follow_slots`) e o nome/interfaces podem mudar em tempo de execução. Sai com "goodbye".
- Broadcast: `BroadcastResponder` na 47802, só responde `PGDISC1` exato (7 bytes), com teto de 32 respostas/s (a resposta é bem maior que o pedido: sem teto seria um amplificador).
- Dispositivos próximos: `NearbyBrowser`/`NearbyTable` sobre `_padgaming-client._tcp`, com `paired` (via `PairedLookup`), debounce de 500 ms, expiração de 15 s e uma confirmação ativa (`verify`) aos 8 s de silêncio. `on_nearby` é o ponto de ligação do evento `devices:discovered` (T-506).
- Interfaces: só IPv4 privadas, ativas e sem cara de virtuais (Hyper-V, WSL, VirtualBox, VMware, Docker, VPNs, Tailscale...), reavaliadas a cada 10 s; `InterfaceChoice::Only` fixa por nome (Configurações) e `interfaces::available()` lista tudo para a UI. `Discovery::local_addresses()` dá os IPs para a conexão manual.
- Simulador: `padgaming-sim discover [--wait 3s] [--no-mdns|--no-broadcast] [--json]` (as duas buscas viram uma linha por server, com `via`) e `padgaming-sim announce` (anuncia um client). Exemplo do server: `cargo run -p padgaming-net --example discovery_server`.

Testes: unitários (interfaces, TXT/resposta, respondedor, tabela com tempo simulado) e `padgaming-sim/tests/discovery.rs`. Os três testes de mDNS real ficam `#[ignore]` (precisam de multicast na rede) e passaram numa LAN Wi-Fi: `cargo test -p padgaming-sim --test discovery -- --ignored`. Medido: client aparece em ~1 s e some em ~1 s (o `mdns-sd` manda o "goodbye" até no `shutdown`); a expiração de 15 s só tem teste com tempo simulado.

Pontos de atenção:
- O `mdns-sd` limita o tipo de serviço a 15 caracteres por padrão (RFC 6763); `_padgaming-client` tem 17. O daemon é configurado com o teto de 30. **No Android (T-903)** confira se o `NsdManager` aceita esse nome; se não, é preciso renomear o serviço (mudança de protocolo).
- Firewall do Windows: a 47802/UDP e a 5353/UDP precisam de regra (T-306).
