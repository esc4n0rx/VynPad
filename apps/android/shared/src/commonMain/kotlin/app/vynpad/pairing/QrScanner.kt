package app.vynpad.pairing

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier

/**
 * Câmera que lê QR codes. Pede a permissão de câmera ao entrar em composição; avisa o resultado
 * em [onPermission] e entrega cada QR lido em [onCode] (um por vez, sem repetir o mesmo).
 * [attempt] muda para pedir a permissão de novo (depois de o usuário tocar em "Permitir").
 */
@Composable
expect fun QrScanner(onCode: (String) -> Unit, onPermission: (granted: Boolean) -> Unit, attempt: Int, modifier: Modifier = Modifier)

/** Abre os ajustes do app no sistema (para liberar a câmera depois de negada de vez). */
@Composable
expect fun rememberOpenAppSettings(): () -> Unit
