# SPEC — PadGaming

> Versão 0.1 · 2026-09-26 · Status: rascunho aprovado para planejamento

## 1. Visão

Transformar qualquer celular Android em um **controle Xbox 360 nativo** para PC. O jogo não sabe que existe um celular: ele enxerga um controle XInput real, então funciona em **qualquer jogo** que suporte controle de Xbox (Steam, Game Pass, emuladores, launchers).

## 2. Componentes

| Componente | Plataforma | Stack | Fase |
|---|---|---|---|
| **Server desktop** | Windows 10/11 x64 | Rust + Tauri 2 + React/TS/Tailwind | F0–F7 |
| **Server desktop** | Linux (X11/Wayland) | Mesmo app, backend `uinput` | F8 |
| **Client** | Android 8.0+ (API 26+) | Kotlin + Jetpack Compose | F9 |
| **Simulador** | CLI multiplataforma | Rust | F3 (para testar o server sem Android) |

## 3. Objetivos (o que "pronto" significa)

1. Até **4 controles virtuais simultâneos**, cada um vinculado a um celular (limite do XInput).
2. **Latência toque → jogo < 10 ms no p95** em Wi-Fi 5 GHz na mesma LAN; processamento no server < 1 ms p99.
3. **Descoberta automática**: o server aparece no celular e os celulares próximos aparecem no server, sem digitar IP.
4. **Pareamento seguro** via QR code (ou PIN), com dispositivos confiáveis lembrados.
5. **Interface moderna estilo macOS Sonoma**: janela arredondada, vidro (Mica/Acrylic), dark mode padrão, onboarding guiado.
6. **Mapeamento por controle**: remapear botões, deadzones, curvas de resposta, sensibilidade, gyro → analógico; perfis reutilizáveis.
7. **Persistência** em SQLite: dispositivos, perfis, configurações, atribuição de slots, histórico de conexão.
8. **Rumble** repassado do jogo para a vibração do celular.
9. **Tray + autostart**: o server roda discreto na bandeja do sistema.
10. **Monitor de latência** por controle (RTT, jitter, perda de pacotes).
11. i18n **PT-BR (padrão) + EN**.

## 4. Requisitos funcionais — Server

| ID | Requisito |
|---|---|
| RF-01 | Criar/destruir controles X360 virtuais via ViGEmBus sob demanda (um por client conectado). |
| RF-02 | Gerenciar 4 slots; permitir fixar dispositivo → slot preferido. |
| RF-03 | Anunciar o serviço via mDNS e responder a broadcast de descoberta. |
| RF-04 | Listar clients próximos (anunciados na rede) com ícone de gamepad e status. |
| RF-05 | Parear via QR code (host, porta, fingerprint do certificado, token de uso único) ou PIN de 6 dígitos. |
| RF-06 | Confiar / renomear / revogar dispositivos. |
| RF-07 | Ao perder conexão: soltar todos os botões imediatamente (estado neutro) em ≤ 250 ms. |
| RF-08 | Reservar o slot por 30 s para reconexão transparente. |
| RF-09 | Editor de mapeamento com visualização ao vivo do controle. |
| RF-10 | Perfis de mapeamento: criar, duplicar, importar/exportar JSON, definir padrão. |
| RF-11 | Repassar rumble (motor grande/pequeno) ao client. |
| RF-12 | Onboarding: boas-vindas → verificação/instalação do ViGEmBus → firewall → primeiro pareamento. |
| RF-13 | Configurações: porta, nome do server, idioma, cor de destaque, autostart, iniciar minimizado, notificações. |
| RF-14 | Monitor de latência em tempo real por controle. |
| RF-15 | Ícone na bandeja com menu (abrir, controles conectados, pausar, sair). |

## 5. Requisitos não funcionais

| ID | Requisito |
|---|---|
| RNF-01 | Latência p95 < 10 ms ponta a ponta (Wi-Fi 5 GHz); server < 1 ms p99. |
| RNF-02 | Uso de CPU ocioso < 1%; com 4 controles ativos < 3% (CPU moderna). |
| RNF-03 | RAM < 150 MB (webview incluída). |
| RNF-04 | Tempo de inicialização até janela visível < 1,5 s. |
| RNF-05 | Nenhum input processado de dispositivo não pareado; pacotes UDP autenticados (AEAD). |
| RNF-06 | Hot path de input sem alocação e fora do runtime async. |
| RNF-07 | UI em 60 fps; animações respeitam "reduzir movimento". |
| RNF-08 | Logs rotativos em arquivo, sem dados sensíveis (tokens). |

## 6. Fora de escopo (v1)

- Internet/WAN (somente LAN). Bluetooth. iOS.
- Emulação de DualShock/DualSense (apenas X360).
- Driver próprio de kernel.
- Streaming de vídeo do PC para o celular.

## 7. Riscos

| Risco | Impacto | Mitigação |
|---|---|---|
| ViGEmBus está arquivado (sem novas versões) | Médio | Driver estável e amplamente usado; manter trait `VirtualPadDriver` isolando o backend. |
| Wi-Fi com power saving no Android aumenta a latência | Alto | `WifiManager.WifiLock` (low latency), envio em thread dedicada, orientar uso de 5 GHz. |
| Firewall do Windows bloqueia UDP/mDNS | Alto | Regra criada no onboarding (com UAC) + diagnóstico na UI. |
| Controles físicos ocupam slots XInput | Médio | Detectar slots ocupados via XInput e avisar na UI. |
| Client Android só vem no final | Médio | Simulador CLI (T-307) desde o marco M1 para testes ponta a ponta. |
| Transparência/vibrancy no Linux é inconsistente | Baixo | Fallback para fundo sólido. |
