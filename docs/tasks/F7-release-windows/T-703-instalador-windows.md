# T-703 — Instalador NSIS + ViGEmBus + assinatura

| Campo | Valor |
|---|---|
| Fase | F7 — Qualidade e release Windows |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-205, T-306 |
| Status | ⬜ A fazer |

## Objetivo
Instalação em um clique, confiável e sem alertas assustadores do SmartScreen.

## Escopo
- Bundler NSIS do Tauri (instalação por usuário, sem admin) + MSI opcional para empresas.
- Setup oficial do ViGEmBus em `resources/`; **não** instalado silenciosamente (o onboarding oferece, conforme D-11).
- Hook do NSIS criando as regras de firewall (reaproveitando o fluxo elevado da T-306) quando o usuário aceita.
- WebView2: modo `downloadBootstrapper` (ou `embedBootstrapper`).
- **Assinatura de código**: certificado OV/EV ou Azure Trusted Signing; assinar o executável, o instalador e o updater.
- Desinstalação: remove as regras de firewall e o autostart; pergunta se mantém os dados (banco/perfis).
- Pipeline `release.yml`: tag `v*` → build → assinatura → GitHub Release com changelog automático.

## Critérios de aceite
- [ ] Instalar e desinstalar numa VM limpa sem sobras (exceto os dados, se o usuário escolher manter).
- [ ] O instalador assinado não dispara o alerta "editor desconhecido".

## Fora de escopo
- Microsoft Store (pós-v1).
