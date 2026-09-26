# T-206 — Pipeline de mapeamento + gyro

| Campo | Valor |
|---|---|
| Fase | F2 — Emulação Windows |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-201 |
| Status | ⬜ A fazer |

## Objetivo
Transformar o input bruto do client no `XReport` final aplicando o perfil do slot: rápido, determinístico e 100% testável.

## Escopo
Crate `padgaming-mapping`, função pura `apply(&CompiledProfile, &RawInput, &mut GyroState, dt) -> XReport`.

Etapas, em ordem:
1. **Sticks**: deadzone interna/externa (radial ou axial), anti-deadzone, curva (linear, exponencial, customizada por 4 pontos), sensibilidade, inverter X/Y, trocar esquerdo ↔ direito.
2. **Gatilhos**: deadzone, ponto de acionamento total, modo digital (gatilho vira botão).
3. **Gyro → stick**: ativo sempre / enquanto um botão é segurado / alternado; alvo (stick direito/esquerdo); sensibilidade X/Y; suavização (filtro 1-Euro); deadzone do gyro; recentralizar; mistura com o stick (soma com clamp).
4. **Botões**: remapeamento livre (qualquer botão → qualquer botão ou nenhum); botão → gatilho total; turbo (P2, frequência configurável).
5. Clamp final para os ranges do XInput.

- `Profile` (serde, JSON versionado `schema_version`) → `CompiledProfile` (tabelas pré-calculadas, LUT de 256/1024 entradas para as curvas).
- Perfil "Padrão" embutido: identidade + deadzone de 8%.

## Detalhes técnicos
- Sem alocação; `apply` < 5 µs (benchmark `criterion`).
- Testes de propriedade (`proptest`): saída sempre dentro do range; perfil identidade = entrada inalterada.

## Critérios de aceite
- [ ] Cobertura de testes ≥ 90% no crate.
- [ ] Benchmark dentro da meta.
- [ ] A troca de perfil em tempo real (ArcSwap) não bloqueia o thread de input.

## Fora de escopo
- A tela de edição (T-604). Persistência (T-402/403).
