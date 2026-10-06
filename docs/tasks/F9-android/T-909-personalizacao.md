# T-909 — Personalização de layout e configurações

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P2 |
| Estimativa | G |
| Depende de | T-905 |
| Status | ✅ Concluída (estilos de tecla e sincronia com o PC ficam como follow-up) |

## Objetivo
Cada jogador ajusta o controle na tela às próprias mãos.

## Escopo
- Editor de layout: arrastar, redimensionar e ajustar a opacidade de cada elemento; esconder elementos; snap em grade.
- Presets: Padrão, Compacto, Canhoto, Só D-pad (retrô).
- Sticks: fixo vs. flutuante, raio, deadzone de toque (independente da deadzone do perfil no server).
- Haptic ao toque: intensidade/desligado. Tema: seguir a cor do slot ou a cor fixa.
- Exportar/importar layouts.
- **Visual personalizado por jogo** ("skin"): ver a seção abaixo.

## Critérios de aceite
- [x] O layout editado persiste e funciona no próximo jogo.
- [x] "Restaurar padrão" sempre disponível.
- [x] Um layout com banner de jogo mantém todos os botões legíveis (o app avisa quando o contraste é ruim) e o gamepad continua fluido.

## Fora de escopo
- Sincronizar o layout (as posições) com o server.

## Visual personalizado por jogo ("skin") — ideia aprovada em 2026-10-05
Hoje o fundo do gamepad é só o grafite do tema. A ideia é o usuário carregar o **banner de um jogo que ele joga** e ter um controle único, no estilo dos controles físicos de edição especial. Implementar junto com o editor de layout, nesta ordem.

### Primeira entrega (essencial)
- **Fundo com imagem:** o usuário escolhe o banner pelo seletor de fotos do Android (Photo Picker, sem pedir permissão de armazenamento). O app reduz a imagem ao tamanho da tela, guarda só no aparelho (nada é enviado para fora) e a desenha como bitmap.
- **Véu de contraste:** escurecimento com controle de intensidade e, opcional, desfoque calculado **uma vez** na importação (nunca por quadro).
- **Cor de destaque extraída da imagem:** a cor dominante do banner vira o accent do gamepad (halo dos botões, brilho do analógico, glow), no lugar do azul do slot. Opção de voltar à cor do slot (P1 azul, P2 verde, P3 laranja, P4 roxo).
- **Um visual por layout:** cada layout salvo guarda seu fundo, véu, cor e opacidade dos botões. Trocar de jogo é trocar de layout, em um toque.
- **Legibilidade garantida:** medir o contraste dos botões sobre a imagem e avisar quando for ruim, com "ajustar automaticamente" (aumenta o véu).

### Depois
- **Estilos de tecla:** vidro (atual), sólido, fosco, só contorno; cor das letras; formato dos botões e estilo do analógico e do direcional.
- **Exportar e importar skins** junto com o layout (arquivo único com o banner reduzido).
- **Sincronia com o PC:** o card do slot no desktop (ver a melhoria "controle vivo") usaria a mesma cor e uma miniatura do banner. O canal de controle limita cada mensagem a 16 KB, então só cabe a cor e uma miniatura pequena: precisa de uma mensagem nova no protocolo (`appearance`), opcional e ignorada por versões antigas (o protocolo manda ignorar tipos desconhecidos).

### Cuidados
- **Desempenho:** decodificar uma vez, no tamanho da tela (sem `inSampleSize` frouxo que estoure a memória), e nunca desfocar em tempo real. O gamepad não pode perder fluidez nem a latência de toque.
- **Direitos autorais:** a imagem fica só no aparelho do usuário; o app **não** traz arte de jogos. Os presets que formos oferecer devem ser originais, sem marcas ou personagens de terceiros.
- **Privacidade:** a imagem nunca sai do aparelho, exceto na exportação explícita feita pelo usuário.

## Entregue (2026-10-06)
Ajustes persistentes (`AppSettings`, SharedPreferences); editor de layout (mover com ímã na grade, tamanho, opacidade, esconder, restaurar peça); layouts múltiplos (criar, renomear, excluir, trocar pelo menu de pausa); modelos Padrão, Compacto, Canhoto e Só direcional; analógicos fixo/flutuante com curso e zona morta de toque; intensidade do haptic de toque e cor do controle por jogador ou fixa; visual por jogo (banner pelo Photo Picker, véu, desfoque calculado na importação, cor extraída, opacidade dos botões, aviso de legibilidade com ajuste automático); exportar/importar `.vynlayout` com a imagem embutida e validação do arquivo.

## Follow-up (não entregue)
- Estilos de tecla (vidro, sólido, fosco, só contorno), cor das letras e formato dos botões.
- Mensagem `appearance` no protocolo para o card do slot no desktop usar a cor e a miniatura do banner.
- Rotação EXIF das imagens importadas é ignorada.
