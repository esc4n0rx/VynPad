# vynpad-client-core

Núcleo do client VynPad (D-26). O app Compose Multiplatform chama este crate por
[UniFFI](https://mozilla.github.io/uniffi-rs/) e, assim, usa o mesmo `vynpad-protocol` do server.

## O que expõe

| Item | Para quê |
|---|---|
| `ClientSession` | Sessão UDP: sela `Input`, `InputMotion` e `Ping`; abre `Pong` e `Rumble` (anti-replay incluso) |
| `parse_pairing_uri` | Lê a URI `vynpad://pair?...` do QR |
| `parse_discovery_response`, `discovery_request`, `discovery_port` | Descoberta por broadcast |
| `core_version`, `protocol_version` | Versões, exibidas em Ajustes → Sobre |
| `CoreError` | Erros com os códigos estáveis dos test vectors |

Sockets, TLS e o laço de envio a 250 Hz entram nas etapas T-903 a T-906.

## Testes

`cargo test -p vynpad-client-core` confere o núcleo contra `protocol/test-vectors/`
(o primeiro input da sessão sai byte a byte igual ao vector `input_neutral`).

## Build para Android

O Gradle (`apps/android`) faz tudo: `./gradlew :androidApp:assembleDebug`.

1. `generateCoreBindings` compila a biblioteca do computador e roda o `uniffi-bindgen`
   (feature `bindgen`) em modo biblioteca, gerando o Kotlin em `shared/build/generated/uniffi`.
2. `buildCoreAndroid` roda `cargo ndk` para `arm64-v8a`, `armeabi-v7a` e `x86_64` e põe as `.so`
   em `androidApp/build/rustJniLibs`.

Pré-requisitos: `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`,
`cargo install cargo-ndk`, NDK 27.1 e `ANDROID_HOME` definido.
