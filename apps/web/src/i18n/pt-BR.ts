export const ptBR = {
  meta: {
    title: "VynPad: o controle do seu PC no seu celular",
    description:
      "Transforme o celular em um controle Xbox virtual para o Windows. Wi-Fi local, baixa latência, vibração, gyro e layout do seu jeito. Sem conta, sem nuvem.",
    ogAlt: "VynPad, o celular como controle do PC",
  },
  nav: {
    label: "Navegação principal",
    features: "Recursos",
    how: "Como funciona",
    tech: "Engenharia",
    download: "Baixar",
    faq: "Perguntas",
    menuOpen: "Abrir menu",
    menuClose: "Fechar menu",
    home: "VynPad, página inicial",
  },
  theme: {
    label: "Tema",
    system: "Automático",
    dark: "Escuro",
    light: "Claro",
  },
  locale: { label: "Idioma", pt: "Português", en: "English" },
  hero: {
    eyebrowVersion: "Versão {version}",
    eyebrowSoon: "Primeira versão em breve",
    title: "O controle do seu PC está no seu bolso.",
    lead: "O VynPad transforma o celular em um controle Xbox virtual para o Windows. Pelo Wi-Fi de casa, com latência baixa, vibração, gyro e um layout desenhado do seu jeito.",
    ctaDownload: "Baixar para {os}",
    ctaDownloadGeneric: "Baixar o VynPad",
    ctaHow: "Ver como funciona",
    note: "Sem conta. Sem nuvem. Tudo acontece na sua rede.",
    scene: {
      player: "Jogador {n}",
      desktopAlt:
        "Janela do VynPad no Windows com o celular A063 conectado como Jogador 1, com latência de 8 ms e conexão excelente.",
      phoneAlt:
        "O controle na tela do celular Android: analógicos, direcional, botões A, B, X e Y, ombros e gatilhos, em uso.",
      phoneLabel: "Controle na tela do celular",
      packetLabel: "Pacotes cifrados pelo Wi-Fi",
      caption: "Capturas reais do VynPad no Windows e no Android",
    },
  },
  proof: {
    label: "Fatos técnicos",
    items: [
      "UDP a até 250 Hz",
      "TLS com certificado fixado",
      "ChaCha20-Poly1305",
      "Núcleo em Rust",
      "Sem conta, sem nuvem",
    ],
  },
  how: {
    eyebrow: "Como funciona",
    title: "Do download ao primeiro jogo em poucos minutos.",
    steps: [
      {
        title: "Instale no PC",
        body: "O VynPad prepara o driver do controle virtual e libera o firewall só para a rede local. Um assistente de cinco passos cuida de tudo.",
      },
      {
        title: "Pareie com um QR",
        body: "Abra o app no celular e aponte para o código na tela do PC. Se preferir, digite um PIN de 6 dígitos. O celular guarda a identidade do PC e só conversa com ele.",
      },
      {
        title: "Jogue",
        body: "Os jogos enxergam um controle Xbox 360 comum. Nada para configurar dentro de cada jogo.",
      },
    ],
    qrLabel: "Exemplo ilustrativo de código de pareamento",
    pinLabel: "ou use o PIN",
    wizardSteps: ["Boas-vindas", "Driver", "Firewall", "Parear", "Pronto"],
  },
  features: {
    eyebrow: "Recursos",
    title: "Tudo o que um controle precisa. E o que só um celular tem.",
    latency: {
      title: "Latência que você não sente",
      body: "O servidor processa cada pacote em menos de 0,1 ms e o botão aparece no XInput em menos de 1 ms, medidos em loopback. A meta ponta a ponta no Wi-Fi 5 GHz é ficar abaixo de 10 ms, e a medição com câmera de 240 fps está em andamento.",
      chartLabel: "Gráfico ilustrativo da latência de um controle ao longo do tempo",
      p99: "p99 no servidor",
      goal: "Meta ponta a ponta",
    },
    players: {
      title: "Até 4 controles ao mesmo tempo",
      body: "Cada celular ganha um slot com cor própria. Troque de lugar, pause ou desconecte com um clique.",
    },
    gyro: {
      title: "Gyro de mira ou de volante",
      body: "Gire o celular como um volante e o analógico acompanha o ângulo, ideal para simuladores e corrida. Ou use a mira por movimento nos jogos de tiro.",
      modeTilt: "Inclinação",
      modeRate: "Velocidade",
      angle: "Ângulo",
    },
    rumble: {
      title: "A vibração do jogo, na sua mão",
      body: "O rumble do jogo vira vibração do celular, com intensidade ajustável. Quando o jogo para, a vibração para junto.",
    },
    layout: {
      title: "Seu layout, seu jogo",
      body: "Mova, redimensione e esconda cada botão. Use o banner do seu jogo como fundo, com a cor de destaque tirada da imagem, e guarde um visual por jogo.",
      names: ["Corrida", "Tiro", "Retrô"],
    },
    privacy: {
      title: "Privado por padrão",
      body: "Sem conta, sem nuvem, sem anúncios. O pareamento fixa o certificado do PC no celular e o input viaja cifrado. As imagens do seu layout nunca saem do aparelho.",
    },
  },
  tech: {
    eyebrow: "Engenharia",
    title: "Um protocolo enxuto, escrito para ser rápido.",
    lead: "O input não passa por intermediários. É um estado de controle em um pacote UDP cifrado, entregue ao servidor em Rust e traduzido para o driver do controle virtual.",
    points: [
      {
        title: "Na hora, sem relógio",
        body: "Cada mudança sai imediatamente, até 250 vezes por segundo, e o estado se repete a 10 Hz como sinal de vida.",
      },
      {
        title: "Cifrado de ponta a ponta",
        body: "O canal de controle usa TLS com o certificado do PC fixado no pareamento. O input vai por UDP com ChaCha20-Poly1305 e um nonce que nunca se repete.",
      },
      {
        title: "Nada de botão preso",
        body: "Se o Wi-Fi falhar por 250 ms, o controle virtual solta tudo sozinho. A sessão volta sem você mexer em nada.",
      },
    ],
    packet: {
      title: "Anatomia de um pacote",
      header: "Cabeçalho",
      headerFields: "magic · versão · tipo · sessão · seq",
      payload: "Estado do controle",
      payloadFields: "botões · gatilhos · analógicos · gyro",
      tag: "Autenticação",
      bytes: "{n} bytes",
      encrypted: "cifrado",
    },
    table: {
      title: "O que medimos",
      rows: [
        { label: "Processamento no servidor (p99)", value: "0,039 ms" },
        { label: "Pacote UDP até o XInput (p99, loopback)", value: "0,58 ms" },
        { label: "CPU com 4 controles", value: "1,5 %" },
        { label: "Memória na bandeja do sistema", value: "23 MB" },
      ],
      note: "Medido em 04/10/2026, no Windows 11, com o driver real. Loopback: sem o Wi-Fi no meio. A latência ponta a ponta com o celular ainda está sendo medida.",
    },
  },
  download: {
    eyebrow: "Download",
    title: "Baixe o VynPad",
    lead: "O servidor roda no Windows e o controle é o app Android. Instale os dois e pareie.",
    recommended: "Recomendado para o seu sistema",
    windows: {
      name: "VynPad para Windows",
      requirements: "Windows 10 ou 11, 64 bits",
      button: "Baixar o instalador",
      altMsi: "Instalador MSI (para empresas)",
    },
    android: {
      name: "VynPad para Android",
      requirements: "Android 8.0 ou superior",
      play: "Baixar na Google Play",
      soon: "Em breve na Google Play",
      soonNote: "Estamos terminando os testes antes de publicar.",
    },
    linux: {
      name: "VynPad para Linux",
      requirements: "Servidor para distribuições com glibc",
      soon: "Em breve",
      soonNote: "Está no roteiro, depois do Windows e do Android.",
    },
    meta: {
      version: "Versão",
      published: "Publicado em",
      size: "Tamanho",
      sha: "SHA-256",
      copy: "Copiar",
      copied: "Copiado",
      notes: "Ver as novidades",
    },
    verify: "Confira o SHA-256 depois de baixar para ter certeza de que o arquivo é o publicado.",
    empty: "A primeira versão pública ainda não foi publicada. Volte em breve.",
    error: "Não conseguimos consultar a versão mais recente agora. Tente de novo em instantes.",
    unavailable: "Esse download não está disponível no momento.",
    prerelease: "Pré-lançamento",
  },
  faq: {
    eyebrow: "Perguntas frequentes",
    title: "O que costumam perguntar.",
    items: [
      {
        q: "Preciso de conta ou de internet?",
        a: "Não. O celular e o PC só precisam estar na mesma rede local. O VynPad não tem conta, nuvem nem anúncios.",
      },
      {
        q: "Quais jogos funcionam?",
        a: "Qualquer jogo que aceite um controle Xbox (XInput), o que cobre a grande maioria dos jogos de PC. Jogos que só aceitam teclado e mouse não se aplicam.",
      },
      {
        q: "É seguro?",
        a: "O celular fixa o certificado do PC no pareamento e o input viaja cifrado. Mesmo assim, só pareie em redes em que você confia. Alguns anti-cheats de jogos online podem não aceitar controles virtuais: confira as regras do jogo.",
      },
      {
        q: "E a latência, na prática?",
        a: "Depende da sua rede. Use o Wi-Fi de 5 GHz e, se puder, o PC ligado por cabo ao roteador. O app mostra a latência de cada controle em tempo real.",
      },
      {
        q: "Funciona no iPhone, no Linux ou no macOS?",
        a: "Ainda não. O servidor roda no Windows e o controle é um app Android. Linux está no roteiro; iPhone e macOS não têm data.",
      },
      {
        q: "O que o gyro faz?",
        a: "Ele pode mover um analógico de dois jeitos: por velocidade, para mirar, ou por inclinação, para girar o celular como um volante em simuladores e jogos de corrida.",
      },
    ],
  },
  cta: {
    title: "Pronto para jogar com o que já está no seu bolso?",
    body: "Instale no PC, pareie em segundos e comece.",
    button: "Ir para o download",
  },
  footer: {
    tagline: "O celular como controle do PC.",
    product: "Produto",
    legal: "Legal",
    privacy: "Privacidade",
    changelog: "Novidades",
    source: "Código no GitHub",
    rights: "© {year} VynPad. Todos os direitos reservados.",
  },
  privacy: {
    title: "Política de privacidade",
    updated: "Atualizada em {date}",
    intro:
      "O VynPad transforma o seu celular em um controle para o VynPad no seu PC. Esta política descreve o que os aplicativos e este site fazem com os seus dados.",
    sections: [
      {
        title: "Não coletamos dados",
        body: "Os aplicativos do VynPad não têm conta, não enviam informações para nós nem para terceiros e não usam ferramentas de análise, anúncios ou relatório de falhas.",
      },
      {
        title: "Rede",
        body: "O app conversa apenas com o PC que você pareou, na sua rede local. A conexão de controle é cifrada com TLS e autenticada pelo certificado do PC fixado no pareamento, e o input viaja cifrado.",
      },
      {
        title: "Dados guardados no seu aparelho",
        body: "O pareamento (o token fica protegido pelo Android Keystore), as suas configurações, os layouts do controle e as imagens que você escolher como fundo. As imagens ficam só no aparelho e saem dele apenas se você exportar um layout. Backup na nuvem e transferência entre aparelhos estão desligados. Desinstalar o app apaga tudo.",
      },
      {
        title: "Permissões do app Android",
        body: "Câmera: usada somente para ler o QR de pareamento; nenhuma imagem é gravada ou enviada, e o pareamento por PIN funciona sem ela. Vibração: rumble do jogo e retorno de toque. Rede e Wi-Fi: falar com o PC e descobri-lo na rede local. Sensores: giroscópio e acelerômetro, só enquanto o gyro está ligado no PC. A seleção de imagem de fundo usa o seletor de fotos do sistema e não pede acesso ao armazenamento.",
      },
      {
        title: "Este site",
        body: "O site não usa cookies de rastreamento nem ferramentas de análise. Guardamos apenas, no seu navegador, as preferências de tema e de idioma que você escolher. Ao baixar um instalador, você é levado ao arquivo publicado no GitHub, que tem a própria política de privacidade.",
      },
      {
        title: "Crianças",
        body: "Os aplicativos não são direcionados a crianças e não coletam dados de ninguém.",
      },
      {
        title: "Mudanças e contato",
        body: "Se esta política mudar, a data acima será atualizada. Dúvidas podem ser enviadas pelo canal de suporte indicado na página do app na Google Play.",
      },
    ],
  },
  changelog: {
    title: "Novidades",
    lead: "As versões publicadas do VynPad, da mais recente para a mais antiga.",
    empty: "Ainda não há versões publicadas.",
    error: "Não conseguimos carregar as versões agora. Tente de novo em instantes.",
    published: "Publicado em {date}",
    prerelease: "Pré-lançamento",
    noNotes: "Sem notas para esta versão.",
    github: "Ver no GitHub",
  },
  notFound: {
    title: "Página não encontrada",
    body: "O endereço não existe ou mudou de lugar.",
    back: "Voltar ao início",
  },
  a11y: { skip: "Pular para o conteúdo" },
};

export type Dictionary = typeof ptBR;
