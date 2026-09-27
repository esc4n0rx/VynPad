# Registro de Decisões (ADR-lite)

Cada decisão tem contexto curto e consequência. Para mudar uma decisão, adicione uma nova entrada que a substitua, sem apagar a antiga.

| # | Decisão | Alternativas consideradas | Motivo |
|---|---|---|---|
| D-01 | **Tauri 2 + Rust** no server | Electron, .NET/WinUI | Binário leve, Rust no hot path de input, multiplataforma (Windows + Linux). |
| D-02 | **React + TypeScript + Tailwind** (shadcn/ui, Radix, Motion) | Svelte, Vue, Solid | Maior ecossistema; shadcn/Radix + Motion aceleram a UI estilo macOS. |
| D-03 | **ViGEmBus** para emular o X360 no Windows (crate `vigem-client`) | Driver próprio KMDF/VHF | Padrão de mercado e compatível com qualquer jogo XInput. Está arquivado, então fica isolado atrás da trait `VirtualPadDriver`. |
| D-04 | **UDP binário** para input + **TCP/TLS** para controle | Só WebSocket; UDP + Bluetooth | UDP evita head-of-line blocking; pacote com o estado completo tolera perda. TCP/TLS para pareamento/config. |
| D-05 | **Pareamento por QR code / PIN** + certificado com pinning + token por dispositivo | Aceitar na UI; rede aberta | Segurança sem atrito: o QR carrega o fingerprint do certificado (sem MITM); UDP autenticado com chave de sessão. |
| D-06 | **Descoberta via mDNS/DNS-SD** + fallback por broadcast UDP | Só broadcast; IP manual | mDNS é nativo no Android (NsdManager); o broadcast cobre redes que filtram multicast. IP manual fica como último recurso. |
| D-07 | **SQLite via `sqlx`** (migrations embutidas), acesso só pelo backend Rust | `tauri-plugin-sql` no frontend | Uma fonte de verdade no Rust; o core precisa ler perfis sem passar pela webview. |
| D-08 | Mapeamento **aplicado no server** (client envia input bruto + sensores) | Mapear no client | Configuração centralizada na UI do server; o client continua simples; os perfis ficam no SQLite. |
| D-09 | Visual **vidro estilo Sonoma** (Mica no Win11, Acrylic no Win10) via `window-vibrancy`; **dark padrão**; accent **#0A84FF** | Dark sólido; neon | Pedido do produto; o fallback sólido cobre Linux e máquinas com transparência desativada. |
| D-10 | **i18n PT-BR (padrão) + EN** com `react-i18next` | Só PT-BR | Infraestrutura desde o início sai mais barata que adaptar depois. |
| D-11 | ViGEmBus **detectado e instalado pelo onboarding** (setup oficial incluído no pacote) | Silencioso no MSI; manual | Transparente para o usuário e com consentimento explícito (UAC). |
| D-12 | Linux com **a mesma UI Tauri**, backend `uinput` | Headless | Um só produto; trait de driver comum. |
| D-13 | Client Android em **Kotlin + Jetpack Compose** | Flutter, Tauri Mobile | Menor latência de toque, acesso direto a sensores, vibração e WifiLock. |
| D-14 | Meta de latência **p95 < 10 ms** | < 20 ms | Jogável em jogos de ação; realista em LAN 5 GHz. |
| D-15 | Hot path de input em **thread dedicada (std)**, fora do Tokio, com MMCSS "Games" no Windows | Tudo em Tokio | Latência previsível, sem jitter do scheduler async. |
| D-16 | **Simulador CLI** (`padgaming-sim`) antes do Android | Esperar o app | Permite validar o server ponta a ponta desde o marco M1. |
| D-17 | **Test vectors compartilhados** (`protocol/test-vectors/*.json`) entre Rust e Kotlin | Testes separados | Garante compatibilidade binária entre as duas implementações. |
| D-18 | Extras no escopo v1: **rumble, tray + autostart, monitor de latência, gyro → analógico** | — | Escolhidos pelo produto. |
| D-19 | **`tracing`** como sistema único de logs (arquivo com rotação diária + terminal); `tauri-plugin-log` removido | `tauri-plugin-log` | Spans por sessão, rotação com retenção de 7 arquivos, captura dos logs do crate `log` via `tracing-log`; o frontend loga por um command próprio. |
