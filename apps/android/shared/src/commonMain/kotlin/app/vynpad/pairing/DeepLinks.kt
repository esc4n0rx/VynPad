package app.vynpad.pairing

import kotlinx.coroutines.flow.MutableStateFlow

/** Links `vynpad://pair?...` recebidos de fora do app (QR lido pela câmera nativa, por exemplo). */
object DeepLinks {
    /** URI de pareamento esperando para ser tratada; a UI limpa quando consome. */
    val pairingUri = MutableStateFlow<String?>(null)
}
