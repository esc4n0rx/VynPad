# T-907 — Rumble → vibração

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P1 |
| Estimativa | P |
| Depende de | T-906 |
| Status | ⬜ A fazer |

## Objetivo
Sentir no celular a vibração do jogo.

## Escopo
- Receber o pacote `0x20` na mesma socket UDP; confirmar devolvendo o `rumble_id` em `rumble_ack_id` no InputState (PROTOCOLO §6.4).
- `VibratorManager`/`Vibrator` com `VibrationEffect.createOneShot` e amplitude (`hasAmplitudeControl`); combinar motor grande e pequeno em uma amplitude (ex.: `max(large, small * 0.6)`).
- Manter vibrando enquanto o valor for ≠ 0 (renovar o efeito a cada ~100 ms); parar ao receber 0 ou após 2 s sem pacote S→C válido (PROTOCOLO §9).
- Intensidade configurável e opção de desligar no app.

## Critérios de aceite
- [ ] "Testar vibração" no server faz o celular vibrar.
- [ ] A vibração para em ≤ 150 ms após o jogo zerar o rumble.

## Fora de escopo
- Haptics avançados (padrões).
