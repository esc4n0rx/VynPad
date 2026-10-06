# T-907 — Rumble → vibração

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P1 |
| Estimativa | P |
| Depende de | T-906 |
| Status | ✅ Concluída (validada no aparelho) |

## Objetivo
Sentir no celular a vibração do jogo.

## Escopo
- Receber o pacote `0x20` na mesma socket UDP; confirmar devolvendo o `rumble_id` em `rumble_ack_id` no InputState (PROTOCOLO §6.4).
- `VibratorManager`/`Vibrator` com `VibrationEffect.createOneShot` e amplitude (`hasAmplitudeControl`); combinar motor grande e pequeno em uma amplitude (ex.: `max(large, small * 0.6)`).
- Manter vibrando enquanto o valor for ≠ 0 (renovar o efeito a cada ~100 ms); parar ao receber 0 ou após 2 s sem pacote S→C válido (PROTOCOLO §9).
- Intensidade configurável e opção de desligar no app.

## Critérios de aceite
- [x] "Testar vibração" no server faz o celular vibrar.
- [x] A vibração para em ≤ 150 ms após o jogo zerar o rumble.

## Implementação
- O núcleo Rust (T-906) já devolve o `rumble_ack_id` e repassa `on_rumble`; nada mudou nele.
- `RumbleMix` (common) combina os motores: `max(large, small * 0.6)` × força (Suave 50 %, Média 75 %, Forte 100 %); rumble ativo nunca vira 0 por arredondamento.
- `RumbleVibrator` (Android): `VibratorManager`/`Vibrator`, `createOneShot` de 150 ms renovado a cada 100 ms; ao chegar `0` chama `cancel()` na hora. Sem `hasAmplitudeControl`, vibra com amplitude padrão. Watchdog: 2 s sem pacote S→C (pong ou rumble) para a vibração; `reset()` ao fim da sessão.
- Ajustes: ligar/desligar, intensidade e "Testar vibração" (local) na tela de Ajustes. Ainda **não persistem** entre execuções (como o tema e o haptics); entram na persistência junto da T-909.

## Fora de escopo
- Haptics avançados (padrões).
