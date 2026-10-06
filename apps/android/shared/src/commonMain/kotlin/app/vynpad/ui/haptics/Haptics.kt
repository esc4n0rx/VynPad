package app.vynpad.ui.haptics

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.staticCompositionLocalOf

/** Vocabulário de haptics do app; cada plataforma mapeia para o que tiver de melhor. */
interface Haptics {
    /** Toque leve ao pressionar um controle. */
    fun press()

    /** Mudança de seleção (segmented, toggle, troca de aba). */
    fun tick()

    /** Ação concluída (conexão, pareamento). */
    fun confirm()

    /** Ação recusada ou erro. */
    fun reject()
}

object NoHaptics : Haptics {
    override fun press() = Unit
    override fun tick() = Unit
    override fun confirm() = Unit
    override fun reject() = Unit
}

val LocalHaptics = staticCompositionLocalOf<Haptics> { NoHaptics }

/** Haptics da plataforma; respeita o ajuste do usuário (`enabled`). */
@Composable
expect fun rememberPlatformHaptics(enabled: Boolean): Haptics

@Composable
fun ProvideHaptics(enabled: Boolean, content: @Composable () -> Unit) {
    CompositionLocalProvider(LocalHaptics provides rememberPlatformHaptics(enabled), content = content)
}
