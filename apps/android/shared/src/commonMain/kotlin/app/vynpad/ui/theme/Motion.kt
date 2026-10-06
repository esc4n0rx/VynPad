package app.vynpad.ui.theme

import androidx.compose.animation.core.FiniteAnimationSpec
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import kotlin.math.sqrt

/**
 * Spring padrão do design system: `stiffness 380, damping 32` (Motion/Framer, massa 1).
 * O Compose pede a razão de amortecimento: 32 / (2·√380) ≈ 0,82.
 */
private const val Stiffness = 380f
private const val Damping = 32f
val VynDampingRatio: Float = Damping / (2f * sqrt(Stiffness))

fun <T> vynSpring(): FiniteAnimationSpec<T> = spring(dampingRatio = VynDampingRatio, stiffness = Stiffness)

/** Variante para valores visuais pequenos (escala, alfa), com precisão menor para parar cedo. */
fun <T> vynSpring(visibilityThreshold: T): FiniteAnimationSpec<T> =
    spring(dampingRatio = VynDampingRatio, stiffness = Stiffness, visibilityThreshold = visibilityThreshold)

/** Troca de páginas: fade + 8 dp de deslocamento em 180 ms. */
const val PageTransitionMillis = 180
const val PageTransitionOffsetDp = 8

fun <T> pageTween(): FiniteAnimationSpec<T> = tween(PageTransitionMillis)

/** Uma mola mais mole para respirar (glow, pulsos). */
fun <T> breathe(): FiniteAnimationSpec<T> = spring(dampingRatio = Spring.DampingRatioLowBouncy, stiffness = Spring.StiffnessVeryLow)
