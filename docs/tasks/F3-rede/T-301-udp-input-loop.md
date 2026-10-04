# T-301 — Loop UDP de input (hot path)

| Campo | Valor |
|---|---|
| Fase | F3 — Rede |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-102, T-203, T-206 |
| Status | ✅ Concluída |

## Objetivo
Receber o input dos celulares e entregá-lo ao driver com a menor latência e o menor jitter possíveis.

## Escopo
- Thread dedicada `input-rx` com `std::net::UdpSocket` bloqueante na porta 47801 (bind `0.0.0.0` e `[::]` quando disponível).
- Windows: `AvSetMmThreadCharacteristicsW("Games")` + `SetThreadPriority(ABOVE_NORMAL)`; buffer de recepção de 256 KB.
- Por pacote: header → lookup `session_id` (tabela fixa de 4 entradas) → `ReplayWindow` → `open` AEAD → mapping → `driver.update()`.
- Atualizar `last_seen` e o endereço de origem da sessão (suporta troca de IP/porta, ex.: roaming de Wi-Fi).
- Pacotes Ping → Pong imediato (com `server_proc_us`).
- Contadores atômicos: recebidos, descartados (tag, replay, sessão desconhecida), processados; histograma do tempo de processamento.
- Modo dev (`--insecure-dev`, só em debug builds): chave fixa, para o M1 funcionar antes da T-302.
- Shutdown limpo (socket com timeout de 100 ms para checar a flag de parada).

## Detalhes técnicos
- Zero alocação no loop; buffer `[u8; 1500]` reutilizado.
- Somente o thread de input escreve no pad do slot; a UI envia comandos por canal.

## Critérios de aceite
- [x] Tempo de processamento p99 < 1 ms, medido pelo histograma, com 4 simuladores a 250 Hz. Medido em loopback: 3000 pacotes, zero descartes, p50 ≈ 33 µs, p99 ≈ 82 µs (limite superior da faixa do histograma), máximo 344 µs.
- [x] Pacotes adulterados/repetidos são descartados e contados.
- [x] Nenhuma alocação no loop: `tests/input_no_alloc.rs` passa 3000 datagramas (Input, Input+Motion e Ping) por `InputEngine::handle` com um allocator contador e conta zero (a alocação única do primeiro pacote, que publica o endereço de origem, fica fora da medição).

## Fora de escopo
- Handshake (T-302). Timeouts (T-304).

## Implementação
- `vynpad-net::input`: `InputEngine` (núcleo sem socket: cabeçalho → sessão pelo `session_id` numa tabela fixa de 8 entradas → replay + AEAD → mapeamento → `SlotManager::submit`; Ping → Pong com `t_server_rx_us` e `server_proc_us`), `InputServer` (thread `input-rx`, comandos `add_session`/`remove_session` por canal, parada em até `poll_interval`) e `InputStats` (contadores atômicos + histograma de 164 faixas, erro ≤ 25%).
- `SessionActivity` expõe `idle_for()` e o último endereço de origem por sessão, para os timeouts da T-304. O endereço acompanha a troca de IP/porta (roaming) e o Pong segue o novo endereço.
- Windows: MMCSS ("Games") e prioridade `ABOVE_NORMAL` na thread (`input/platform.rs`, o único `unsafe` da crate, com duas chamadas FFI documentadas). O erro `ConnectionReset` do UDP no Windows (ICMP de um envio anterior) é ignorado.
- Modo dev (só em debug): `InputServer::add_insecure_dev_session` + `DEV_SESSION_ID`/`DEV_KEY`. A flag `--insecure-dev` de linha de comando fica para quem montar o processo (T-302/T-307).
- Diferença em relação ao texto: o socket escuta só IPv4 (`0.0.0.0`), porque o protocolo (§4.2) só anuncia IPv4; `[::]` não foi ligado.
- O critério de 4 simuladores usa 4 threads-cliente com espera ativa (o timer do Windows tem ~15 ms de resolução); o simulador de verdade é a T-307.
