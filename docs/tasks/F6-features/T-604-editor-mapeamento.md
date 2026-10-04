# T-604 — Editor de mapeamento e perfis

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P0 |
| Estimativa | G (pode virar GG; quebrar em 604a Botões/Sticks e 604b Gyro/Perfis se preciso) |
| Depende de | T-206, T-403, T-505 |
| Status | 🟨 Pronta; falta conferir o visual no app, exportar/importar entre duas máquinas e a visão 3D do gyro, o ponto bruto × mapeado do stick (dependem de dados que o backend ainda não expõe) |

## Objetivo
Configurar cada controle de forma visual, com feedback em tempo real.

## Escopo
- Layout em 3 colunas: **lista de perfis** | **gamepad interativo** (T-505) | **painel de propriedades**.
- Seletor "Pré-visualizar com: P1..P4" → input ao vivo do slot mostrado **antes e depois** do mapeamento (ex.: o stick exibe o ponto bruto e o ponto mapeado).
- Propriedades ao clicar em:
  - **Botão**: mapear para (select com ícones), turbo.
  - **Stick**: gráfico de deadzone interna/externa (círculos arrastáveis), editor de curva (preset + pontos arrastáveis), sensibilidade, inverter X/Y, trocar esquerdo ↔ direito.
  - **Gatilho**: deadzone, ponto máximo, modo digital.
  - **Gyro** (aba própria): ativação, alvo, sensibilidade X/Y, suavização, deadzone, botão de recentralizar, visualização 3D simples da orientação do celular.
  - **Rumble**: intensidade, desligar.
- Perfis: criar, duplicar, renomear, ícone, excluir, importar/exportar `.padprofile`, "Restaurar padrão".
- Salvamento automático com debounce de 300 ms + desfazer (Ctrl+Z) na sessão de edição.

## Critérios de aceite
- [ ] Alterar a deadzone reflete no jogo em < 1 s, sem reconectar.
- [ ] Exportar e importar o perfil em outra máquina resulta em configuração idêntica.
- [ ] Todos os controles são acessíveis por teclado.

## Fora de escopo
- Macros e combos (pós-v1).

## Fechamento
- `features/mapping`, em três colunas: `ProfileList` (criar, duplicar, exportar, excluir com confirmação e números de uso, importar `.padprofile`, restaurar o padrão), o controle interativo (T-505) com o seletor "Pré-visualizar com: P1..P4" (o desenho segue o input ao vivo do slot) e as propriedades (`ControlPanels`: botão, analógico, gatilho; `GyroPanel` em aba própria).
- `profileModel.ts` concentra as regras (padrões do backend, atualizações imutáveis, limites de validação do backend: deadzones somam < 0,95, ponto máximo do gatilho acima da deadzone, turbo de 1 a 30 Hz, pontos da curva com `x` crescente) e é testado sem React. `evalCurve` espelha o `eval_curve` do backend (retas entre os pontos).
- `useProfileEditor`: rascunho local, salvamento com debounce de 300 ms (o backend aplica na hora nos controles que usam o perfil, sem reconectar), desfazer com Ctrl+Z (arrastar um slider vale um passo só, até 100 passos) e gravação pendente ao sair da tela. O perfil embutido é só de leitura (o backend recusa gravá-lo): a tela avisa e oferece duplicar.
- Deadzone: diagrama com duas alças arrastáveis e sliders (o caminho por teclado). Curva: presets linear e exponencial, e personalizada com 4 pontos que se arrastam ou se movem com as setas.
- Exportar usa `Blob` + `<a download>` e importar usa `<input type="file">`: não precisou do plugin de diálogo do Tauri, mas o "Salvar como" da webview ainda precisa ser conferido no app.
- Ficou de fora, por depender de dados que o backend ainda não expõe: o ponto **bruto** do stick ao lado do mapeado (só o estado já mapeado é emitido), e a visualização 3D do gyro (não há stream de movimento). A aba "Vibração" (liga/desliga e intensidade de 0 a 200%) veio com a T-204.
