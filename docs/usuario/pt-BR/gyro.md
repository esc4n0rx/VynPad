# Gyro: mirar mexendo o celular {#gyro}

O gyro usa o sensor de movimento do celular para mover um analógico, como em jogos de console com mira por movimento. Bom para FPS e jogos de mira.

## Ligar {#ligar}

Em **Mapeamento → Gyro**, ligue **Ativar o gyro**. O celular passa a mandar o movimento junto com os botões (só se o aparelho tiver o sensor).

## Dois jeitos de usar {#resposta}

Em **Como o movimento age** você escolhe:

- **Velocidade (mira)**: o analógico se move *enquanto* o celular gira e volta ao centro quando ele para. É o jeito dos jogos de console com mira por movimento.
- **Inclinação (volante)**: o analógico acompanha o **ângulo** do celular e **fica onde ele está**. Gire o celular como um volante e o analógico esquerdo vai junto, com precisão. É o certo para simuladores e jogos de corrida (Euro Truck Simulator, por exemplo).

Ao escolher **Inclinação**, o perfil já se arruma como volante (analógico esquerdo, eixo X, girar como um volante, sem suavização). Dá para mudar depois. O **ângulo até o fim do curso** diz quantos graus levam o analógico ao máximo (45° por padrão: menos graus = volante mais sensível). O botão de **recentralizar** passa a fixar a posição atual do celular como o ponto neutro.

O ângulo vem da gravidade medida pelo celular, corrigida pelo giroscópio, então aguenta bem a trepidação. Se o celular for deitado (como uma bandeja), use o movimento **Virar para os lados**.

## Quando age {#quando-age}

- **Sempre**: o gyro move o analógico o tempo todo.
- **Segurando**: só enquanto você segura um botão (por exemplo LB). Bom para mira fina.
- **Alternar**: cada toque no botão liga ou desliga.

## Ajustes {#ajustes}

- **Analógico alvo**: esquerdo ou direito (em geral o direito, que mira a câmera).
- **Movimento do eixo X e Y**: virar para os lados, inclinar para cima e para baixo ou girar como um volante.
- **Sensibilidade X e Y**: quanto o analógico anda para cada movimento. Comece em 1,0.
- **Inverter** X ou Y.
- **Deadzone do gyro**: ignora tremidas pequenas da mão.
- **Suavização**: um filtro que tira o tremor sem atrasar movimentos rápidos. A **frequência de corte** menor suaviza mais; a **reação a movimentos rápidos** maior reduz o atraso.
- **Botão de recentralizar**: zera o filtro e desliga o modo alternado.
- **Somar ao analógico**: o gyro se soma ao que o analógico já faz (com limite). Desligado, o gyro substitui o analógico enquanto age.

## Dicas {#dicas-gyro}

- Segure o celular com firmeza e use as duas mãos.
- Se a mira "foge" sozinha, aumente a deadzone do gyro ou use o botão de recentralizar.
- A mira muito lenta ou muito rápida é ajuste de **sensibilidade**; o atraso é outra coisa, veja [Latência](#latencia).
