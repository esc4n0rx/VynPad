package app.vynpad.ui

import androidx.compose.runtime.Composable

/** Trata o botão/gesto de voltar do sistema enquanto `enabled`. */
@Composable
expect fun PlatformBackHandler(enabled: Boolean, onBack: () -> Unit)

/** Enquanto `active`: tela em landscape e barras do sistema escondidas (gamepad imersivo). */
@Composable
expect fun ImmersiveLandscapeEffect(active: Boolean)
