# T-601 — Onboarding

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P0 |
| Estimativa | G |
| Depende de | T-205, T-306, T-603 |
| Status | ⬜ A fazer |

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
