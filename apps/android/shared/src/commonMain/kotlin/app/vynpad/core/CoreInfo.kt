package app.vynpad.core

/** Versões do núcleo Rust (`vynpad-client-core`) carregado no app. */
data class CoreInfo(val version: String, val protocolVersion: Int)

/** Lê as versões do núcleo nativo; `null` se a biblioteca não pôde ser carregada. */
expect fun loadCoreInfo(): CoreInfo?
