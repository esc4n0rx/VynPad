# T-604 — Editor de mapeamento e perfis

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P0 |
| Estimativa | G (pode virar GG; quebrar em 604a Botões/Sticks e 604b Gyro/Perfis se preciso) |
| Depende de | T-206, T-403, T-505 |
| Status | ⬜ A fazer |

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
