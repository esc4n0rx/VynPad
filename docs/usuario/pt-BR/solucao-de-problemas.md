# Solução de problemas {#solucao-de-problemas}

Se nada aqui resolver, vá em **Configurações → Sobre → Exportar diagnóstico**: o app gera um arquivo `.zip` (sem senhas, tokens nem chaves) para você enviar ao suporte.

## O celular não encontra o PC {#o-celular-nao-encontra-o-pc}

1. Confira que o celular e o PC estão na **mesma rede Wi-Fi** (nada de rede de visitantes, que isola os aparelhos).
2. Em **Configurações → Rede**, veja o endereço do PC e use **Testar conexão**.
3. Se o firewall estiver bloqueando, clique em **Corrigir** (veja [O firewall bloqueia](#o-firewall-bloqueia)).
4. Se a rede estiver como **pública**, mude para privada (veja [Rede pública](#rede-publica)).
5. VPNs, WSL e Hyper-V criam redes virtuais que confundem a descoberta. Em **Configurações → Rede**, as interfaces marcadas como **Virtual** não são usadas.
6. Como último recurso, digite o IP do PC no celular (a conexão manual).

## O firewall bloqueia {#o-firewall-bloqueia}

O Windows precisa deixar o celular entrar nas portas do VynPad. Em **Configurações → Rede → Firewall**, clique em **Corrigir** (ou **Permitir conexões**, na introdução). O Windows pede permissão de administrador uma vez e o app cria as regras só para redes **Privada** e **Domínio**. Se você recusar a permissão, dá para tentar de novo.

## Rede pública {#rede-publica}

Em uma rede marcada como **pública** o Windows bloqueia conexões de entrada, e as regras do VynPad não valem nela. Abra as configurações de rede (o app tem o botão) e mude a rede para **privada**.

## O ViGEmBus não aparece {#vigembus}

Sem o **ViGEmBus**, o VynPad não cria controles. Veja o estado em **Configurações → Controles** (ou no passo **Driver** da introdução):

- **Não instalado**: use **Baixar o ViGEmBus**, instale e volte. O app percebe sozinho.
- **Desatualizado**: instale a versão indicada ou mais nova.
- **Instalado, mas parado**: reinicie o PC ou o serviço "ViGEmBus" nos Serviços do Windows.

## O jogo não reconhece o controle {#o-jogo-nao-reconhece-o-controle}

- Confirme que o controle aparece conectado no painel e que os botões mexem o desenho.
- Teste em **joy.cpl** (Configurar controladores de jogo do Windows). Se lá funciona, o problema é do jogo.
- Alguns jogos só aceitam o controle se estiver ligado **antes** de o jogo abrir.
- O Windows só enxerga **quatro controles XInput** ao todo, somando os físicos e os virtuais. Desligue um controle que não use.
- Jogos antigos (DirectInput) podem exigir um tradutor como o x360ce.

## O Steam Input duplica o controle {#steam-input}

O Steam pode enxergar o controle virtual e também o que o jogo vê, e o jogo recebe dois. Em **Steam → Configurações → Controle → Configurações gerais do controle**, desligue o **Suporte para controle Xbox**, ou, no jogo, escolha **Desativar o Steam Input** nas propriedades.

## Input atrasado {#input-atrasado}

Veja a página [Latência](#latencia): use Wi-Fi de 5 GHz, desligue a economia de bateria do Wi-Fi do celular e confira o RTT e a perda em **Latência**.

## Algo deu errado {#err-internal}

Um erro inesperado. Tente de novo. Se continuar, exporte o diagnóstico (**Configurações → Sobre**) e envie ao suporte: o arquivo mostra o que aconteceu.

## Um valor não é válido {#err-invalid-input}

Algum campo recebeu um valor fora do permitido (uma porta abaixo de 1024, por exemplo). A mensagem diz qual. Corrija o valor e tente de novo.

## Recurso indisponível {#err-unsupported}

A ação só existe no Windows (o driver ViGEmBus e o firewall do Windows, por exemplo). Neste sistema ela não está disponível.

## Item não encontrado {#err-not-found}

O dispositivo, perfil ou controle que você usou já não existe (alguém o removeu ou a conexão acabou). Atualize a tela e tente de novo.

## Perfil Padrão não pode ser editado {#err-builtin-profile}

O perfil **Padrão** é do app e fica fixo. Duplique-o (menu do perfil → **Duplicar**) e edite a cópia. Para voltar ao original, use **Restaurar padrão**.

## Perfil em uso {#err-profile-in-use}

O perfil que você quer apagar está sendo usado por dispositivos ou controles conectados. O app mostra quantos e pede confirmação; se você confirmar, eles passam para o perfil **Padrão**.

## Arquivo de perfil inválido {#err-import-invalid}

O arquivo `.padprofile` está corrompido, é grande demais ou não é de um perfil do VynPad. Peça outro à pessoa que o exportou, ou exporte de novo.

## Sem rede local {#err-no-network}

O PC não está em nenhuma rede local, então não há como o celular alcançá-lo. Conecte o PC ao Wi-Fi ou à rede cabeada e tente de novo.

## Permissão do Windows recusada {#err-cancelled}

Você fechou ou recusou o pedido de permissão de administrador do Windows. A ação não foi feita. Tente de novo e clique em **Sim** quando o Windows perguntar.
