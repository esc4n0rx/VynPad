# T-301 — Loop UDP de input (hot path)

| Campo | Valor |
|---|---|
| Fase | F3 — Rede |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-102, T-203, T-206 |
| Status | ⬜ A fazer |

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
- [ ] Tempo de processamento p99 < 1 ms, medido pelo histograma, com 4 simuladores a 250 Hz.
- [ ] Pacotes adulterados/repetidos são descartados e contados.
- [ ] Nenhuma alocação no loop (teste com allocator contador em modo bench).

## Fora de escopo
- Handshake (T-302). Timeouts (T-304).
