# Testes E2E da interface

WebdriverIO + `tauri-driver`, no Windows (WebView2). O app abre de verdade e o `vynpad-sim` faz o
papel do celular. Cenários em `specs/` (rodam em ordem e compartilham a pasta de dados):

1. `01-onboarding`: pasta de dados nova abre na introdução e vai até o dashboard.
2. `02-pairing`: lê o PIN na tela, o simulador pareia pelo canal TLS e a UI mostra o aparelho; PIN errado é recusado.
3. `03-profiles`: duplica o perfil Padrão e o define para o dispositivo pareado.

## Rodar

Uma vez:

```text
cargo install tauri-driver --locked
pnpm --filter @vynpad/e2e drivers        # msedgedriver da versão do seu WebView2
```

A cada mudança no app ou no simulador:

```text
pnpm --filter @vynpad/desktop build
cargo build --release -p vynpad-desktop --features tauri/custom-protocol
cargo build -p vynpad-sim
pnpm --filter @vynpad/e2e e2e
```

Feche o VynPad antes (o app aceita uma instância só). Para um arquivo só:
`pnpm --filter @vynpad/e2e exec wdio run wdio.conf.mjs --spec specs/02-pairing.e2e.mjs`.
Os cenários 02 e 03 dependem do estado deixado pelo 01; para repeti-los, aponte
`VYNPAD_DATA_DIR` para uma pasta que já tenha passado pela introdução.

## Isolamento

O app roda com `VYNPAD_DATA_DIR` numa pasta temporária (apagada no fim) e `VYNPAD_E2E=1`:
portas **47900–47902** em vez das padrão, sem anúncio mDNS e sem procurar atualização. Os dados e
o VynPad do usuário não são tocados.

## Variáveis

`VYNPAD_E2E_APP` (caminho do app), `VYNPAD_SIM` (simulador), `TAURI_DRIVER`, `EDGE_DRIVER`.
Quando um teste falha, a captura de tela vai para `e2e/logs/`.

## Escrevendo cenários

`lib/ui.mjs` tem `byText` (o `*=` do WebdriverIO só acha **links**; use XPath), `pressAny`,
`goTo` e `chooseMenuItem`. Num menu Radix, escolha o item pelo teclado: o clique do WebDriver
teletransporta o ponteiro e o submenu fecha.
