# QA — checklist manual (T-701)

O que os testes automáticos **não** cobrem: jogos reais, o ViGEmBus real, o instalador numa
máquina limpa e o celular. Execute antes de **cada release** e registre o resultado na tabela do
fim. O que um teste automático já prova está em [Estratégia de testes](#estratégia-de-testes).

> Regra: um item que falha bloqueia o release, a menos que vire uma task e você registre a decisão
> de seguir mesmo assim.

## 0. Preparação

- [ ] Máquina de teste: Windows 11 (e, se der, um Windows 10), **sem** VynPad nem PadGaming instalados.
- [ ] Um celular Android com o app do VynPad (depois da F9) **ou** o simulador:
      `cargo run -p vynpad-sim -- pair --pin <PIN> --host <IP do PC> --hold 600s`.
- [ ] PC e celular na mesma rede; de preferência Wi-Fi de 5 GHz.
- [ ] Versão a testar anotada (`Configurações → Sobre`) e o instalador **assinado** do release.

## 1. Instalação e desinstalação (T-703)

- [ ] O instalador abre **sem** o alerta "editor desconhecido" do SmartScreen (assinatura válida).
- [ ] Instala por usuário, sem pedir administrador; cria atalho no Menu Iniciar.
- [ ] Primeira abertura: aparece a introdução; o app abre em menos de 2 s.
- [ ] Desinstalar **mantendo** os dados: sem sobras no Menu Iniciar, no registro (`Run`) e nas
      regras de firewall `VynPad *`; `%APPDATA%\VynPad` continua.
- [ ] Desinstalar **apagando** os dados: `%APPDATA%\VynPad` e `%LOCALAPPDATA%\VynPad` somem.
- [ ] Reinstalar por cima da mesma versão e de uma versão mais antiga não perde pareamentos.

## 2. Introdução, driver e firewall

- [ ] Sem o ViGEmBus: a introdução avisa, oferece o link oficial e deixa "Continuar sem driver".
- [ ] Instalando o ViGEmBus com o app aberto: o app percebe sozinho (sem reiniciar).
- [ ] Firewall: "Permitir conexões" pede o UAC e cria as quatro regras; recusar o UAC não quebra nada.
- [ ] Rede pública: o aviso aparece e "Abrir configurações de rede" abre a tela certa do Windows.

## 3. Pareamento e conexão

- [ ] Pareamento por **QR code** e por **PIN**; o fingerprint mostrado confere com o do celular.
- [ ] PIN errado cinco vezes trava por um tempo e mostra a mensagem correta.
- [ ] "Pedir aprovação" ligado: o primeiro pareamento espera o "Permitir"; "Negar" recusa.
- [ ] Revogar um dispositivo o desconecta na hora e ele não volta sem parear de novo.
- [ ] Quatro celulares nos quatro slots; o quinto é recusado com "sem slot livre".
- [ ] Celular perde o Wi-Fi e volta em menos de 30 s: reconecta no mesmo slot, com o mesmo perfil.
- [ ] Desligar o Wi-Fi do celular por mais de 30 s: o slot é liberado.
- [ ] Fechar o app: o celular mostra "desconectado" e não fica pendurado.

## 4. Jogos (ViGEmBus real)

Para cada jogo, confira: o jogo reconhece **um** controle Xbox 360, os botões e analógicos
respondem, os gatilhos são analógicos, e a vibração chega ao celular.

| Jogo / cenário | O que olhar | Resultado |
|---|---|---|
| Jogo da **Steam com Steam Input** | Não duplica o controle (ver "Steam Input" na ajuda); botões certos no Big Picture | |
| Jogo do **Game Pass / UWP** | O controle é reconhecido; sem atraso perceptível | |
| Emulador (**RetroArch**) | Mapeia os botões; D-pad e analógico; hotkeys | |
| Jogo de **luta** (latência) | Combos sem input perdido; latência p95 < 10 ms no monitor de latência | |
| **FPS com gyro** | Mira por movimento suave, sem deriva; ligar/desligar o gyro no perfil | |

- [ ] Um controle físico no PC ao mesmo tempo: o app o identifica ("Controle físico") e não o confunde com um slot.
- [ ] Rumble: `Testar vibração` no dashboard vibra o celular; o jogo vibra e para ao fechar.

## 5. Mapeamento e perfis

- [ ] Trocar o perfil de um slot em partida muda o comportamento na hora, sem reconectar.
- [ ] Duplicar, renomear, exportar e importar um perfil; restaurar o Padrão.
- [ ] Zona morta, curvas, gatilhos, turbo e gyro mudam o que o jogo recebe.

## 6. Janela, bandeja e notificações

- [ ] Fechar a janela deixa o app na bandeja (com o aviso da primeira vez); o menu da bandeja tem Abrir e Sair.
- [ ] "Iniciar com o Windows" e "Iniciar minimizado" funcionam depois de reiniciar o PC.
- [ ] Notificações: conexão, pareamento e driver; **silêncio durante jogo em tela cheia**.
- [ ] Tema claro/escuro, idioma pt-BR/en e "reduzir movimento" valem na hora.
- [ ] RAM e CPU na bandeja (meta: < 150 MB e < 1% de CPU ocioso).

## 7. Atualização (T-704)

- [ ] `Procurar atualizações` acha a versão nova publicada e mostra as novidades.
- [ ] **Com um controle conectado**, "Atualizar agora" pede confirmação e só instala depois de confirmar.
- [ ] Depois de atualizar: banco, perfis e pareamentos continuam; o celular reconecta sozinho.
- [ ] Com a opção "Procurar atualizações sozinho" desligada, o app não consulta o GitHub ao abrir.

## 8. Diagnóstico e ajuda

- [ ] `Exportar diagnóstico` gera o `.zip` **sem** token, chave nem PIN (abra e confira).
- [ ] A ajuda abre pelo botão, por `F1` e pelos links de erro; vale em pt-BR e en.

---

## Estratégia de testes

| Camada | Ferramenta | Onde roda |
|---|---|---|
| Unit Rust | `cargo test`, `proptest` | CI (Windows e Linux) |
| Integração Rust | `tokio::test` + `MockDriver` + `vynpad-sim` em processo (handshake, sessão, timeout, reconexão, rumble, pareamento) | CI |
| Fuzz | `cargo-fuzz` (decode de pacote, frame de controle) + estresse por mutação | `fuzz.yml` e `cargo test` |
| Cobertura | `cargo-llvm-cov`, mínimo 80% de linhas nos crates de domínio | CI (job `coverage`) |
| Unit frontend | Vitest + Testing Library | CI |
| E2E da UI | WebdriverIO + `tauri-driver` + `vynpad-sim` como celular (`e2e/`) | CI (Windows, job `e2e`) |
| Hardware | testes `#[ignore]` do ViGEmBus e do XInput; benchmark (T-702) | manual, na máquina com o driver |
| Manual | este checklist | antes de cada release |

Como rodar os testes de hardware:

```text
cargo test -p vynpad-driver --test vigem_hardware -- --ignored --test-threads=1
cargo test -p vynpad-server --release --test xinput_latency -- --ignored --nocapture --test-threads=1
```

## Registro das execuções

| Versão | Data | Quem | Testado em | Resultado | Notas / itens que falharam |
|---|---|---|---|---|---|
| | | | | | |
