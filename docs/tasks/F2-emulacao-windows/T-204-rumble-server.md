# T-204 — Rumble: driver → client

| Campo | Valor |
|---|---|
| Fase | F2 — Emulação Windows |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-202, T-301 |
| Status | 🟨 Pronta; falta ver o rumble do XInput Test chegando ao `padgaming-sim` com o ViGEmBus real e conferir o "Testar vibração" com um celular |

## Objetivo
Repassar a vibração pedida pelo jogo para o celular do slot correspondente.

## Escopo
- Thread `rumble-tx` consome `driver.rumble_events()`.
- Resolve slot → sessão → endereço UDP do client + `SessionCipher`.
- Envia o pacote `0x20` com um `rumble_id` novo quando o valor muda e retransmite o estado atual a cada 100 ms até o client devolver `rumble_ack_id` igual ao `rumble_id` atual (PROTOCOLO §6.4 e §8). Isso vale também para `0,0`, o que garante o desligamento mesmo com perda.
- Configuração por perfil: intensidade (0–200%) e desligar rumble.
- Teste de vibração pela UI (botão "Testar vibração" no card do slot).

## Critérios de aceite
- [ ] Com o simulador, o rumble do "XInput Test" aparece no log do `padgaming-sim`.
- [ ] Com 10% de perda simulada, o motor nunca fica "preso" ligado.
- [ ] O botão de teste funciona.

## Fora de escopo
- Vibração no Android (T-907).

## Fechamento
- A vibração é enviada **pela própria thread `input-rx`**, e não por uma thread `rumble-tx` separada como a spec previa: o `Session` do servidor numera os pacotes enviados (`seq`) e o `Pong` já sai dali, então um segundo remetente competiria pelo mesmo contador. `InputEngine::set_rumble` guarda o estado (`rumble_id` novo a cada mudança, nunca `0`) e `send_rumble` manda o estado novo e o reenvia a cada 100 ms até o `rumble_ack_id` do client igualar o `rumble_id` atual. Um `ack` de estado antigo não encerra nada, e `0, 0` também é um estado, então desligar chega mesmo com perda de pacotes.
- No `Core`, a thread `rumble-pump` lê `driver.rumble_events()`, resolve slot → sessão e aplica a intensidade do perfil do slot (`CompiledProfile::scale_rumble`) antes de chamar `InputServer::set_rumble`.
- Perfil: nova seção `rumble { enabled, intensity }` (0–200%, satura em 255; perfis antigos sem a seção seguem valendo). `Config.rumble_enabled` do client reflete o perfil e é reenviado quando ele muda; um perfil que desliga o rumble (ou a troca para um) para na hora o que estava vibrando.
- "Testar vibração": `Core::test_rumble` faz o celular vibrar a 100% por meio segundo e para sozinho; está no menu `…` do card do slot (vale mesmo com o rumble desligado no perfil: testa o caminho até o celular). A aba "Vibração" do editor de mapeamento ajusta o liga/desliga e a intensidade.
- Testes: UDP em loopback (envio, retransmissão a cada ~100 ms, `ack`, novo `rumble_id`, desligar, perda simulada, `ack` antigo) e no `Core` com o simulador (intensidade, perfil mudo, config e o teste de vibração).
