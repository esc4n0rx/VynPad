package app.vynpad.ui.screens

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.dp
import app.vynpad.resources.Res
import app.vynpad.resources.connect_tagline
import app.vynpad.ui.components.VynIcon
import app.vynpad.ui.components.VynIconView
import app.vynpad.ui.components.VynText
import app.vynpad.ui.haptics.LocalHaptics
import app.vynpad.ui.theme.VynTheme
import app.vynpad.ui.theme.vynSpring
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import org.jetbrains.compose.resources.stringResource

private const val Wordmark = "VynPad"

/**
 * Abertura do app (~1,9 s): o ícone do app pousa com a mola do design system, três ondas de
 * accent saem dele como o radar da descoberta, o nome aparece letra a letra e tudo some em fade
 * para a tela principal. Um haptic confirma o pouso do ícone.
 */
@Composable
fun SplashScreen(onFinished: () -> Unit, modifier: Modifier = Modifier) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    val haptics = LocalHaptics.current

    val logoScale = remember { Animatable(0.55f) }
    val logoAlpha = remember { Animatable(0f) }
    val glow = remember { Animatable(0f) }
    val waves = remember { List(3) { Animatable(0f) } }
    val letters = remember { List(Wordmark.length) { Animatable(0f) } }
    val tagline = remember { Animatable(0f) }
    val exit = remember { Animatable(1f) }

    LaunchedEffect(Unit) {
        coroutineScope {
            async { logoAlpha.animateTo(1f, tween(260)) }
            async { glow.animateTo(1f, tween(900)) }
            async { logoScale.animateTo(1f, vynSpring()) }
            async {
                delay(300L)
                haptics.confirm()
            }
            waves.forEachIndexed { i, wave ->
                async {
                    delay(260L + i * 190L)
                    wave.animateTo(1f, tween(1100, easing = LinearEasing))
                }
            }
            letters.forEachIndexed { i, letter ->
                async {
                    delay(520L + i * 55L)
                    letter.animateTo(1f, vynSpring())
                }
            }
            async {
                delay(900L)
                tagline.animateTo(1f, tween(380))
            }
        }
        delay(250L)
        exit.animateTo(0f, tween(320, easing = FastOutSlowInEasing))
        onFinished()
    }

    Box(
        modifier
            .fillMaxSize()
            .graphicsLayer {
                alpha = exit.value
                val s = 1f + (1f - exit.value) * 0.06f
                scaleX = s
                scaleY = s
            },
        contentAlignment = Alignment.Center,
    ) {
        Canvas(Modifier.fillMaxSize()) {
            val center = Offset(size.width / 2f, size.height / 2f - 40.dp.toPx())
            val reach = size.maxDimension * 0.55f
            drawCircle(
                Brush.radialGradient(
                    listOf(c.accent.copy(alpha = 0.30f * glow.value), c.indigo.copy(alpha = 0.10f * glow.value), androidx.compose.ui.graphics.Color.Transparent),
                    center,
                    reach,
                ),
                reach,
                center,
            )
            waves.forEach { w ->
                if (w.value > 0f && w.value < 1f) {
                    drawCircle(
                        c.accent.copy(alpha = (1f - w.value) * 0.55f),
                        radius = 44.dp.toPx() + (reach - 44.dp.toPx()) * w.value,
                        center = center,
                        style = Stroke(2.dp.toPx()),
                    )
                }
            }
        }

        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(22.dp)) {
            Box(
                Modifier
                    .graphicsLayer { alpha = logoAlpha.value }
                    .scale(logoScale.value)
                    .size(104.dp)
                    .clip(RoundedCornerShape(30.dp))
                    .background(Brush.linearGradient(listOf(c.accent, c.indigo))),
                contentAlignment = Alignment.Center,
            ) {
                VynIconView(VynIcon.Gamepad, tint = c.onAccent, size = 56.dp)
            }
            Row {
                Wordmark.forEachIndexed { i, ch ->
                    VynText(
                        ch.toString(),
                        style = t.hero.copy(),
                        modifier = Modifier.graphicsLayer {
                            alpha = letters[i].value.coerceIn(0f, 1f)
                            translationY = (1f - letters[i].value) * 28f
                        },
                    )
                }
            }
            VynText(
                stringResource(Res.string.connect_tagline),
                style = t.body,
                color = c.textSecondary,
                modifier = Modifier.graphicsLayer { alpha = tagline.value }.padding(top = 0.dp),
            )
        }
    }
}
