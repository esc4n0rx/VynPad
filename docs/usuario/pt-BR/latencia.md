# Latência: o input está atrasado? {#latencia}

A **latência** é o tempo entre tocar no celular e o jogo reagir. O PadGaming mede a conexão de cada controle o tempo todo e mostra em **Latência** e no card do painel.

## O que a tela mostra {#o-que-a-tela-mostra}

- **RTT** (p50 e p95): o tempo de ida e volta até o celular. O p95 é o que você sente nos piores momentos.
- **Jitter**: o quanto o intervalo entre os envios oscila. Jitter alto é "engasgo".
- **Perda**: quantos pacotes não chegaram.
- **Pacotes/s**: a taxa de envio (250 por segundo é o normal).
- **No server**: o tempo que o PC leva para entregar o input ao driver.
- **Qualidade**: Excelente, Boa, Instável ou Ruim, pela pior das medidas.

O gráfico mostra o RTT de cada controle nas cores dos slots, em janelas de 30 s, 2 min ou 10 min. Dá para pausar, esconder um controle pela legenda e copiar um resumo em texto para pedir ajuda.

## Como melhorar {#como-melhorar}

1. Use o **Wi-Fi de 5 GHz** e fique perto do roteador. É a maior diferença.
2. Desative a **economia de bateria do Wi-Fi** no celular (ela faz o rádio dormir entre os pacotes e aumenta o jitter).
3. Feche apps que usam muita rede no PC e no celular (downloads, streaming).
4. Prefira o PC por **cabo** ligado ao roteador: só o celular fica no Wi-Fi.
5. Em **Configurações → Controles**, a taxa de envio de **250 Hz** é mais precisa que 125 Hz; 125 Hz poupa bateria e rede.

Se o RTT está bom (menos de 20 ms) e mesmo assim o jogo parece lento, o atraso pode estar no jogo ou no monitor, não na conexão.
