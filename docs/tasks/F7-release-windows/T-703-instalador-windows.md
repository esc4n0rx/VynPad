# T-703 — Instalador NSIS + ViGEmBus + assinatura

| Campo | Valor |
|---|---|
| Fase | F7 — Qualidade e release Windows |
| Prioridade | P0 |
| Estimativa | M |
| Depende de | T-205, T-306 |
| Status | 🟨 Em andamento |

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
- [ ] Instalar e desinstalar numa VM limpa sem sobras (exceto os dados, se o usuário escolher manter). Testado em silencioso na máquina de desenvolvimento: instala, e o desinstalador remove executável, atalho, chave de desinstalação e a entrada de "Iniciar com o Windows". Falta a VM limpa e o caminho interativo (UAC das regras de firewall e a caixa de apagar dados).
- [ ] O instalador assinado não dispara o alerta "editor desconhecido". Depende de um certificado ou de uma conta do Azure Trusted Signing.

## Implementação
- `tauri.conf.json` (`bundle`): NSIS por usuário sem admin, pt-BR e en com seletor de idioma, MSI em pt-BR e en-US, WebView2 por `downloadBootstrapper`.
- `apps/desktop/src-tauri/windows/hooks.nsh`: no desinstalador remove as regras de firewall `VynPad *` (UAC só se existirem; nunca em modo silencioso), a entrada de autostart e, se o usuário marcar a opção, `%APPDATA%\VynPad` e `%LOCALAPPDATA%\VynPad`.
- `.github/workflows/release.yml`: tag `v*` confere a versão, compila NSIS e MSI, assina com o Azure Trusted Signing quando os segredos existem (`AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `TRUSTED_SIGNING_ENDPOINT`, `TRUSTED_SIGNING_ACCOUNT`, `TRUSTED_SIGNING_PROFILE`), confere as assinaturas e publica o GitHub Release com changelog automático. Sem os segredos publica sem assinar e avisa.

## Decisões
- **ViGEmBus não vai no pacote.** A T-205 decidiu levar o usuário ao download oficial, e o `THIRD-PARTY-NOTICES.md` registra que o app não redistribui o driver. Isso substitui o item "setup em `resources/`" do escopo original.
- As regras de firewall são criadas pelo app (fluxo elevado da T-306), não pelo instalador; o instalador só as remove.

## Fora de escopo
- Microsoft Store (pós-v1).
