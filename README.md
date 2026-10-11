<div align="center">

# VynPad

**Transforme o celular Android em um controle de Xbox 360 para o PC.**

O jogo não sabe que existe um celular: ele enxerga um controle XInput de verdade.

[![CI](https://github.com/esc4n0rx/VynPad/actions/workflows/ci.yml/badge.svg)](https://github.com/esc4n0rx/VynPad/actions/workflows/ci.yml)
![Windows 10/11](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D4)
![Android 8+](https://img.shields.io/badge/Android-8%2B-3DDC84)
![Rust](https://img.shields.io/badge/Rust-1.98-orange)
![Licença](https://img.shields.io/badge/licen%C3%A7a-propriet%C3%A1ria-lightgrey)

[Documentação](docs/README.md) · [Roadmap](docs/00-visao-geral/ROADMAP.md) · [Protocolo](docs/00-visao-geral/PROTOCOLO.md) · [Segurança](SECURITY.md)

</div>

---

## O que é

O VynPad tem duas partes que conversam pela rede local:

- **Server desktop** (Windows hoje, Linux depois): app de bandeja que cria até **4 controles Xbox 360 virtuais** com o driver ViGEmBus. Funciona em qualquer jogo, launcher ou emulador que aceite controle de Xbox: Steam, Game Pass, RetroArch e outros.
- **Client Android**: o celular vira o controle, com botões, analógicos, gatilhos analógicos, vibração, giroscópio, volante por inclinação e um editor de layout com modos de jogo prontos.

```mermaid
flowchart LR
    subgraph Celular
        A[App Android<br/>toque + gyro]
    end
    subgraph PC
        S[Server VynPad<br/>UDP + TLS]
        V[ViGEmBus]
        G[Jogo<br/>XInput]
    end
    A -- "input (UDP, ChaCha20-Poly1305)" --> S
    A <-. "pareamento e controle (TLS 1.3)" .-> S
    S -- "controle virtual X360" --> V --> G
    G -. "vibração" .-> S -. "rumble" .-> A
```

### Destaques

**No celular (Android)**

- **Pareamento simples:** por QR code ou PIN, com descoberta automática do PC (radar animado) ou IP manual.
- **Controle multitouch** no layout do Xbox: analógicos flutuantes ou fixos, gatilhos analógicos, direcional de 8 vias, ABXY, ombros e retorno tátil.
- **Modos de jogo:** *Padrão*, *Compacto*, *Canhoto*, *Retrô*, **Corrida** (volante no centro, pedais de freio e acelerador, marchas) e **FPS** (mirar e atirar ao toque, correr com trava).
- **Volante:** gira com o dedo e/ou inclinando o celular como um volante de verdade (acelerômetro); vira o eixo X de um analógico, então funciona em qualquer jogo.
- **Editor de layout completo:** mova, redimensione, esconda, duplique e remova peças; adicione botões livres (qualquer botão do controle, com texto e opção de travar), pedais e volante; guarde vários layouts e **exporte/importe** em arquivo `.vynlayout`.
- **Visual por jogo:** use o banner do jogo como fundo, com cor de destaque extraída da imagem e véu para manter a legibilidade.
- **Vibração** do jogo repassada ao celular, com força ajustável, e **gyro** para mira ou direção.
- **Sessão estável:** serviço em primeiro plano ("Conectado como P1") e reconexão automática se a rede cair.

**No PC (Windows)**

- **Latência baixa:** meta de p95 < 10 ms de ponta a ponta no Wi-Fi 5 GHz. Medido hoje, sem o celular: processamento no server em **0,04 ms** (p99) e **0,14 ms** do pacote UDP até o botão aparecer no `XInputGetState`. A medição completa com o celular será registrada em [`docs/benchmarks/`](docs/benchmarks/).
- **Seguro por padrão:** pareamento com *pinning* do certificado, TLS 1.3 no canal de controle, input UDP autenticado e cifrado, nada é processado de um aparelho não pareado. Veja o [modelo de ameaças](docs/SEGURANCA.md).
- **Mapeamento por controle:** remapeamento de botões, zonas mortas, curvas, gatilhos, turbo e gyro (por velocidade ou por inclinação, para volante), com perfis que podem ser exportados e importados.
- **Monitor de latência** por controle: RTT, jitter e perda de pacotes; o card de cada slot "ganha vida" na cor do jogador quando o celular conecta.
- **Reconexão transparente:** o slot fica reservado por 30 s e, sem sinal, os botões são soltos em até 250 ms.
- **Interface moderna** com Mica/Acrylic, tema claro e escuro, bandeja do sistema, PT-BR e EN.

## Status

| Parte | Situação |
|---|---|
| Protocolo, rede, emulação, mapeamento, persistência | ✅ Pronto e testado |
| App desktop para Windows | 🟨 Funcional; instalador e atualização automática implementados. Faltam o primeiro release publicado, a assinatura de código e a validação numa VM limpa |
| App Android | 🟨 Funcional (parear, jogar, rumble, gyro, editor e modos de jogo). Faltam a medição de latência ponta a ponta com o celular e a publicação na Google Play |
| Site oficial (`apps/web`) | 🟨 Pronto; falta hospedar e publicar o primeiro release |
| Linux (`uinput`) | ⬜ Planejado (F8) |
| iOS | ⬜ Planejado: o client é Compose Multiplatform, mas o build exige um Mac |

O quadro completo, task por task, está no [roadmap](docs/00-visao-geral/ROADMAP.md).

## Instalação (usuários)

**No PC**

1. Baixe o instalador da [última versão](https://github.com/esc4n0rx/VynPad/releases/latest) (`VynPad_x.y.z_x64-setup.exe`). A instalação é por usuário e não pede administrador. Enquanto o instalador não tiver assinatura de código, o Windows SmartScreen pode avisar na primeira execução: clique em *Mais informações* e depois em *Executar assim mesmo* (confira o SHA-256 publicado).
2. Abra o VynPad: a introdução guia a instalação do [ViGEmBus](https://vigembusdriver.com/download/), a liberação no firewall e o primeiro pareamento.
3. O app se atualiza sozinho: ele avisa quando há versão nova e **nunca** instala com um controle conectado sem a sua confirmação.

**No celular**

1. Instale o app Android do VynPad (Google Play, em breve).
2. No VynPad do PC, abra *Dispositivos > Parear novo* e leia o QR code (ou digite o PIN) no celular.

Guias de uso: [primeiros passos](docs/usuario/pt-BR/primeiros-passos.md), [parear um celular](docs/usuario/pt-BR/parear-um-celular.md), [mapeamento e perfis](docs/usuario/pt-BR/mapeamento-e-perfis.md), [gyro](docs/usuario/pt-BR/gyro.md), [latência](docs/usuario/pt-BR/latencia.md) e [solução de problemas](docs/usuario/pt-BR/solucao-de-problemas.md).

## Estrutura do repositório

| Pasta | Conteúdo |
|---|---|
| `apps/desktop` | Server desktop: Tauri 2 + React + TypeScript + Tailwind |
| `apps/android` | Client Android: Kotlin + Compose Multiplatform (`shared/` e `androidApp/`), com o núcleo de rede em Rust via UniFFI |
| `apps/web` | Site oficial: Next.js 16, com download dos releases do GitHub e política de privacidade |
| `crates/vynpad-protocol` | Pacotes UDP, mensagens de controle, criptografia (`no_std` no núcleo) |
| `crates/vynpad-client-core` | Núcleo do client (sessão, pareamento, envio de input), exposto ao Kotlin via UniFFI |
| `crates/vynpad-driver` | Controle virtual: ViGEmBus, XInput e `MockDriver` |
| `crates/vynpad-mapping` | Perfis, curvas, zonas mortas, gyro e turbo |
| `crates/vynpad-net` | UDP de input, servidor TLS de controle, descoberta, pareamento, firewall |
| `crates/vynpad-core` | Gerenciador dos 4 slots |
| `crates/vynpad-storage` | SQLite: dispositivos, perfis, configurações e histórico |
| `crates/vynpad-server` | Composição de tudo isso no `Core` usado pelo app |
| `crates/vynpad-sim` | Simulador CLI: faz o papel do celular para testar sem o Android |
| `crates/vynpad-dbprep` | Ferramenta de desenvolvimento: gera os metadados do `sqlx` |
| `protocol/` | Schemas, *test vectors* e implementação de referência, compartilhados entre Rust e Kotlin |
| `e2e/` | Testes de interface (WebdriverIO + `tauri-driver`) |
| `scripts/` | Release (versão, builds assinados, GitHub Release), changelog e verificações |
| `docs/` | Especificação, arquitetura, decisões, roadmap, tasks e guias do usuário |

## Desenvolvimento

Requisitos: Rust (versão fixada em `rust-toolchain.toml`), Node 22 ou mais novo, pnpm e, no Windows, o ViGEmBus para os testes de hardware.

```sh
pnpm install
pnpm --filter @vynpad/desktop tauri dev      # abre o app desktop em modo de desenvolvimento
pnpm --filter @vynpad/web dev                # abre o site em http://localhost:3000
```

Sem o celular, o simulador faz o papel dele:

```sh
cargo run -p vynpad-sim -- pair --pin 123456 --host 127.0.0.1 --hold 60s   # o PIN aparece em Dispositivos > Parear novo
cargo run -p vynpad-sim -- bench --insecure-dev --clients 4 --rate 250     # com o dev_server rodando
```

### App Android

Requisitos: JDK 17, Android SDK (plataforma 36), NDK 27.1, os alvos Rust do Android (`aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android`) e o `cargo-ndk`. Com `JAVA_HOME` e `ANDROID_HOME` definidos:

```sh
cd apps/android
./gradlew :androidApp:assembleDebug          # APK de debug
./gradlew :shared:testAndroid                # testes de lógica
./gradlew :androidApp:lintRelease            # lint do Android
```

O ktlint roda como binário no CI (versão e hash fixados em `.github/workflows/ci.yml`). O build de release, a assinatura e a publicação na Play estão em [`docs/android/PUBLICACAO.md`](docs/android/PUBLICACAO.md).

### Qualidade

Os mesmos comandos do CI:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --exclude vynpad-desktop --locked
cargo test -p vynpad-desktop --locked        # separado: ver a nota no ci.yml (CRT estática do Tauri)
cargo deny check
pnpm format:check && pnpm lint && pnpm typecheck && pnpm test
pnpm --filter @vynpad/protocol check         # test vectors em dia com o gerador
```

Testes de interface, cobertura e hardware:

```sh
pnpm --filter @vynpad/e2e e2e                                    # ver e2e/README.md
cargo llvm-cov --workspace --exclude vynpad-desktop              # mínimo de 80% nos crates de domínio
cargo test -p vynpad-server --release --test xinput_latency -- --ignored --nocapture --test-threads=1
```

A estratégia completa e o checklist manual de release estão em [`docs/QA-CHECKLIST.md`](docs/QA-CHECKLIST.md).

## Contribuindo e lançando versões

- Fluxo de trabalho, convenção de commits e como rodar tudo: [CONTRIBUTING.md](CONTRIBUTING.md).
- **Release automático, da sua máquina:** `scripts/release.sh X.Y.Z` sobe a versão (Cargo, Tauri e Android), roda as verificações, gera o AAB e o APK assinados e o instalador do Windows, e cria o GitHub Release (rascunho, ou publicado com `--publish`). As chaves e senhas ficam fora do repositório; veja `scripts/release.sh --help` e [`docs/LANCAMENTO-E-PENDENCIAS.txt`](docs/LANCAMENTO-E-PENDENCIAS.txt).
- **Pelo CI:** uma tag `vX.Y.Z` igual à versão do `tauri.conf.json` dispara o [release](.github/workflows/release.yml): instalador NSIS e MSI, `latest.json` do auto-update e notas geradas dos commits.
- Vulnerabilidades: veja [SECURITY.md](SECURITY.md). Não abra uma issue pública para elas.

## Licença

Software proprietário. Todos os direitos reservados; veja [LICENSE](LICENSE). O código-fonte está visível neste repositório público, mas isso não concede nenhuma licença de uso. Os componentes de terceiros estão em [docs/THIRD-PARTY-NOTICES.md](docs/THIRD-PARTY-NOTICES.md).
