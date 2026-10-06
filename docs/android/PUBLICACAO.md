# VynPad Android — build de release e publicação (T-910)

O que já está pronto no repositório e o que depende de você. Nada aqui contém chaves ou senhas.

## 1. Build de release

Rodam de `apps/android`, com `JAVA_HOME` (JDK 17) e `ANDROID_HOME` definidos:

```
./gradlew :shared:testAndroid :androidApp:lintRelease     # testes de lógica e lint do Android
./gradlew :androidApp:assembleRelease                      # APK (R8 + shrink de recursos)
./gradlew :androidApp:bundleRelease                        # App Bundle (.aab) que vai para a Play
```

O CI (`ci.yml`, job `android`) roda esses mesmos comandos mais o ktlint (binário fixado por versão e
hash SHA-256) e guarda o APK e o AAB **sem assinatura** como artefato.

O R8 está ligado. As regras ficam em `androidApp/proguard-rules.pro` e cobrem o JNA e os bindings do
UniFFI (chamados por reflexão pelo núcleo Rust), o kotlinx.serialization e os registradores do ML Kit
(o leitor de QR falhava só no release sem eles).

## 2. Assinatura

O repositório não guarda keystore. O `build.gradle.kts` lê quatro variáveis de ambiente; sem elas o
release sai sem assinar:

| Variável | O quê |
|---|---|
| `VYNPAD_KEYSTORE` | caminho do arquivo `.jks` de **upload** |
| `VYNPAD_KEYSTORE_PASSWORD` | senha do keystore |
| `VYNPAD_KEY_ALIAS` | alias da chave |
| `VYNPAD_KEY_PASSWORD` | senha da chave |

Gerar a chave de upload (uma vez; guarde o `.jks` e as senhas em lugar seguro e com cópia offline):

```
keytool -genkeypair -v -keystore vynpad-upload.jks -alias vynpad-upload -keyalg RSA -keysize 4096 -validity 10000
```

Com o **Play App Signing** ativo, a Google guarda a chave de assinatura final; a de upload pode ser
trocada pelo suporte se for perdida. Mesmo assim, não a perca.

## 3. O que depende de você (em ordem)

1. [ ] Criar a conta de desenvolvedor no Google Play Console (taxa única).
2. [ ] Gerar a chave de upload (seção 2) e, se for publicar pelo CI, cadastrar os quatro valores como
   segredos do repositório (o keystore em Base64).
3. [ ] Criar o app no Play Console (`app.vynpad.android`), ativar o Play App Signing e subir o primeiro
   `.aab` na faixa **Teste interno**.
4. [ ] Hospedar a política de privacidade (texto na seção 5) em uma URL pública e informá-la no Console.
5. [ ] Preencher "Segurança dos dados" (seção 4), classificação de conteúdo e público-alvo.
6. [ ] Tirar os screenshots (celular, paisagem) e enviar o ícone de 512 px e o gráfico de destaque.
7. [ ] Convidar testadores (Teste fechado) e acompanhar crashes no Play Console.
8. [ ] Medir a latência ponta a ponta com câmera em pelo menos 3 aparelhos (procedimento da T-702)
   e registrar em `docs/benchmarks/`.
9. [ ] Promover para produção quando a taxa de crash estiver < 0,5 %.
10. [ ] Atualizar o onboarding do desktop (T-601) com o link da Play Store.

## 4. "Segurança dos dados" (Play Console)

- **Coleta de dados:** nenhuma. O app não envia dados a servidores do VynPad nem de terceiros, não tem
  analytics, anúncios nem crash reporting.
- **Compartilhamento:** nenhum.
- **Tráfego de rede:** só local, entre o celular e o PC do próprio usuário (TLS com *pinning* do
  certificado do PC e UDP cifrado com ChaCha20-Poly1305).
- **Dados no aparelho:** o token de pareamento (cifrado pelo Android Keystore), os ajustes, os layouts e
  as imagens de fundo escolhidas pelo usuário. Backup na nuvem e transferência entre aparelhos estão
  desligados (`data_extraction_rules.xml`).

### Permissões e a justificativa de cada uma

| Permissão | Para quê |
|---|---|
| `INTERNET`, `ACCESS_NETWORK_STATE`, `ACCESS_WIFI_STATE` | falar com o PC na rede local |
| `CHANGE_WIFI_MULTICAST_STATE` | descoberta do PC (mDNS) |
| `WAKE_LOCK` | Wi-Fi em modo de baixa latência enquanto conectado |
| `VIBRATE` | rumble do jogo e feedback de toque |
| `CAMERA` | ler o QR de pareamento (opcional: o PIN funciona sem ela) |
| `HIGH_SAMPLING_RATE_SENSORS` | gyro acima de 200 Hz para a mira por movimento |

A seleção de imagem de fundo usa o seletor de fotos do sistema e **não** pede permissão de
armazenamento.

## 5. Política de privacidade (rascunho)

**VynPad — Política de privacidade** (última atualização: preencher na publicação)

O VynPad Android transforma o seu celular em um controle para o VynPad no seu PC. Esta política
descreve o que o app faz com os seus dados.

*Não coletamos dados.* O app não tem conta, não envia informações para nós nem para terceiros, e não
usa ferramentas de análise, anúncios ou relatório de falhas.

*Rede.* O app conversa apenas com o PC que você pareou, na sua rede local. A conexão é cifrada e
autenticada.

*Dados guardados no seu aparelho.* O pareamento (token protegido pelo Android Keystore), as suas
configurações, os layouts do controle e as imagens que você escolher como fundo. Essas imagens ficam só
no aparelho e saem dele apenas se você exportar um layout. Desinstalar o app apaga tudo.

*Câmera.* Usada somente para ler o QR de pareamento; nenhuma imagem é gravada ou enviada.

*Crianças.* O app não é direcionado a crianças e não coleta dados de ninguém.

*Contato.* Preencher com o e-mail de suporte.

## 6. Texto da loja (rascunho)

**PT-BR — título:** VynPad — Controle para PC
**Descrição curta:** Use o celular como controle de Xbox no seu PC, com baixa latência e Wi-Fi local.
**Descrição:** Transforme o celular em um controle para jogar no PC. Pareie com o VynPad no seu computador
por QR ou PIN e jogue com botões, analógicos, gatilhos analógicos, vibração e mira por giroscópio.
Personalize o layout, escolha o banner do seu jogo como fundo e salve um visual por jogo. Tudo roda na
sua rede local: sem conta, sem anúncios, sem coleta de dados. Requer o VynPad instalado no PC (Windows).

**EN — title:** VynPad — PC Gamepad
**Short description:** Turn your phone into a low-latency Xbox controller for your PC over local Wi-Fi.
**Description:** Turn your phone into a PC game controller. Pair with VynPad on your computer by QR or PIN
and play with buttons, sticks, analog triggers, rumble and gyro aiming. Customize the layout, use your
game's banner as the background and keep one look per game. Everything stays on your local network: no
account, no ads, no data collection. Requires VynPad on your PC (Windows).

## 7. Foreground service (pendência da T-906)

Hoje a sessão não roda em serviço em primeiro plano: se o usuário sair do app, o Android pode suspender
a rede. Antes de publicar é preciso decidir o tipo declarado no Play Console (por exemplo
`specialUse`, com justificativa de "controle de jogo em tempo real"), pois a declaração é revisada pela
Google. Veja o status em `docs/tasks/F9-android/T-910-release-android.md`.
