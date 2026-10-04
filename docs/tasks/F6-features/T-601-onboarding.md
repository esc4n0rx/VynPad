# T-601 — Onboarding

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-205, T-306, T-603 |
| Status | 🟨 Pronta; falta o fluxo numa VM Windows limpa com o simulador como celular, e ver o fundo animado e o visual no app |

## Objetivo
Levar um usuário leigo do "acabei de instalar" ao "estou jogando com o celular" em menos de 2 minutos.

## Escopo
Tela cheia, sem sidebar, com stepper e fundo com gradiente animado sutil sobre o vidro.

1. **Boas-vindas**: ícone do app, título "Seu celular é o controle", seletor de idioma (PT-BR / EN).
2. **Driver**: verifica o ViGEmBus. ✅ Pronto → avança. ❌ → botão "Instalar driver" (UAC) com progresso e explicação curta do que é o driver.
3. **Rede**: verifica o firewall e o perfil da rede; botão "Permitir conexões" (T-306); mostra o IP e o nome do PC.
4. **Conectar celular**: QR code grande + PIN alternativo + link para baixar o app Android (QR secundário para a Play Store, quando existir); lista de "dispositivos próximos" ao vivo; ao parear, animação de sucesso com o gamepad acendendo em P1.
5. **Pronto**: resumo e dicas (Wi-Fi 5 GHz, tray, atalhos); opções "Iniciar com o Windows" e "Iniciar minimizado".

- "Pular por enquanto" nos passos 3 e 4 (o passo 2 é obrigatório para emular, mas permite "Continuar sem driver" com aviso).
- Persistir `onboarding_completed`; reabrir por Configurações → Geral → "Refazer introdução".
- Retomar do passo onde parou se o app for fechado.

## Critérios de aceite
- [ ] Numa VM Windows limpa, o fluxo completo funciona com o simulador fazendo o papel do celular.
- [ ] Cada passo trata o erro com mensagem clara e ação de recuperação.
- [ ] Textos em PT-BR e EN.

## Fora de escopo
- Onboarding Linux (T-802).

## Fechamento
- `features/onboarding`: tela cheia com indicador de etapas, fundo de manchas de cor que derivam devagar (`.onboarding-bg`, sem animação com "reduzir movimento") e cinco passos. `steps.ts` guarda a ordem, quais passos se pulam (rede e celular) e onde o usuário parou (`localStorage`, que pode falhar sem derrubar nada); `OnboardingSteps.tsx` tem os passos.
- 1 Boas-vindas com o idioma; 2 Driver (estado do ViGEmBus, por que ele existe, "Baixar o ViGEmBus" e a verificação segue sozinha pelo evento `driver:changed`; sem o driver só há "Continuar sem driver", com aviso); 3 Rede (nome e endereço do server, status do firewall, "Permitir conexões" com UAC e tratamento de recusa, aviso de rede pública); 4 Celular (o mesmo `PairingPanel` do modal de dispositivos, QR e PIN que se renovam, lista de celulares por perto ao vivo e, ao parear, o controle de P1 aceso); 5 Pronto (resumo do que ficou pendente, dicas, "Iniciar com o Windows" e "Iniciar minimizado").
- `onboarding_completed` é gravado ao final; a primeira abertura leva à introdução (`useOnboardingRedirect`, só na leitura inicial das configurações) e Configurações → Geral → "Refazer introdução" a reabre.
- O "Instalar driver (UAC) com progresso" da spec virou "Baixar o ViGEmBus" (página oficial) com detecção automática: o app não redistribui o driver (licença e assinatura), como já decidido na T-205. O QR da Play Store não existe ainda: a tela avisa que o app chega em breve.
