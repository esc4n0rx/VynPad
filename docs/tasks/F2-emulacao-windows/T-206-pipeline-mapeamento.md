# T-206 — Pipeline de mapeamento + gyro

| Campo | Valor |
|---|---|
| Fase | F2 — Emulação Windows |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-201 |
| Status | 🟨 Pronta; falta só medir a cobertura (≥ 90%) |

## Objetivo
Transformar o input bruto do client no `XReport` final aplicando o perfil do slot: rápido, determinístico e 100% testável.

## Escopo
Crate `vynpad-mapping`, função pura `apply(&CompiledProfile, &RawInput, &mut GyroState, dt) -> XReport`.

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
- [x] Benchmark dentro da meta: `apply` leva ~36 ns (perfil padrão) e ~60 ns (todas as etapas ligadas), contra a meta de 5 µs (`cargo bench -p vynpad-mapping`).
- [x] A troca de perfil em tempo real (`ProfileCell`, sobre `ArcSwap`) não bloqueia o thread de input: um teste troca o perfil sem parar enquanto outra thread mapeia e confere que só saem valores dos dois perfis.

## Fora de escopo
- A tela de edição (T-604). Persistência (T-402/403).

## Implementação
- `vynpad-mapping`: `Profile` (JSON com `schema_version`, todos os campos opcionais, validado com o caminho do campo no erro) → `CompiledProfile` (LUTs de 1024 passos; curva linear sem LUT, para a identidade ser exata) → `apply(&CompiledProfile, &RawInput, &mut MapperState, dt) -> XReport`.
- `MapperState` guarda o estado do gyro (filtros 1-Euro, modo alternado, recentralizar) e o relógio do turbo. Diferença em relação ao texto da task: em vez de só `GyroState`, o estado inclui também o turbo.
- `ProfileCell` (troca em tempo real) e `Mapper` (perfil + estado de um slot). `RawInput::from_wire` converte os pacotes UDP (gyro de rad/s×1000 para rad/s).
- Convenções do gyro (referencial do PROTOCOLO §6.4): virar para a esquerda (`gy` > 0) move o analógico para a esquerda; inclinar o topo para longe do jogador (`gx` < 0) mira para cima; sensibilidade 1,0 = fundo de escala a 2 rad/s (~115°/s).
- Deadzone radial não limita a magnitude em 1 (os cantos do quadrado passam), para o perfil identidade devolver a entrada exata; o clamp é por eixo no fim.
- Testes: 17 unitários + 32 de comportamento + 6 propriedades (`proptest`: identidade, totalidade/determinismo, neutro→neutro, monotonia, JSON sem perdas, botões conhecidos) + teste de zero alocação.
- Pendente: medir a cobertura (`cargo llvm-cov` não está instalado nesta máquina).
- Fica para a T-301: converter `InputState`/`Motion` com `RawInput::from_wire`, chamar `Mapper::map` e entregar o `XReport` ao `SlotManager::submit`.
