package app.vynpad.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import app.vynpad.discovery.DiscoveredPc
import app.vynpad.ui.haptics.LocalHaptics
import app.vynpad.ui.theme.VynTheme
import app.vynpad.ui.theme.vynSpring
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin

private const val SweepMillis = 2800
private const val PingMillis = 2400

/**
 * Radar de descoberta: varredura circular em accent, anéis que expandem e cada PC encontrado
 * surge como um ponto que acende quando a varredura passa por ele. Tocar no ponto abre o PC.
 */
@Composable
fun DiscoveryRadar(
    pcs: List<DiscoveredPc>,
    scanning: Boolean,
    onSelect: (DiscoveredPc) -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = VynTheme.colors
    val transition = rememberInfiniteTransition()
    val sweep = transition.animateFloat(0f, 360f, infiniteRepeatable(tween(SweepMillis, easing = LinearEasing)))
    val ping = transition.animateFloat(0f, 1f, infiniteRepeatable(tween(PingMillis, easing = LinearEasing)))
    val active by animateFloatAsState(if (scanning) 1f else 0f, tween(400))

    BoxWithConstraints(modifier.fillMaxWidth().aspectRatio(1f), contentAlignment = Alignment.Center) {
        val radiusPx = with(LocalDensity.current) { maxWidth.toPx() / 2f }

        Canvas(Modifier.fillMaxSize()) {
            val center = Offset(size.width / 2f, size.height / 2f)
            val r = size.minDimension / 2f
            drawCircle(Brush.radialGradient(listOf(c.accent.copy(alpha = 0.16f), Color.Transparent), center, r), r, center)
            val line = Stroke(width = 1.dp.toPx())
            listOf(1f / 3f, 2f / 3f, 1f).forEach { f ->
                drawCircle(c.accent.copy(alpha = if (f == 1f) 0.30f else 0.16f), r * f - 1f, center, style = line)
            }
            drawLine(c.accent.copy(alpha = 0.10f), Offset(center.x - r, center.y), Offset(center.x + r, center.y), line.width)
            drawLine(c.accent.copy(alpha = 0.10f), Offset(center.x, center.y - r), Offset(center.x, center.y + r), line.width)

            if (active > 0.01f) {
                // Varredura: cauda em gradiente atrás da linha líder.
                rotate(sweep.value, center) {
                    drawCircle(
                        brush = Brush.sweepGradient(
                            0f to Color.Transparent,
                            0.70f to Color.Transparent,
                            1f to c.accent.copy(alpha = 0.38f * active),
                            center = center,
                        ),
                        radius = r - 1f,
                        center = center,
                    )
                    drawLine(c.accent.copy(alpha = 0.9f * active), center, Offset(center.x + r - 1f, center.y), 2.dp.toPx())
                }
                listOf(0f, 0.5f).forEach { shift ->
                    val p = (ping.value + shift) % 1f
                    drawCircle(c.accent.copy(alpha = (1f - p) * 0.5f * active), r * (0.12f + 0.88f * p), center, style = Stroke(2.dp.toPx()))
                }
            } else {
                drawCircle(c.accent.copy(alpha = 0.2f), r * 0.22f, center, style = line)
            }
        }

        Box(Modifier.fillMaxSize()) {
            pcs.forEach { pc ->
                key(pc.id) { RadarBlip(pc, radiusPx, sweep, onSelect) }
            }
        }

        Box(
            Modifier
                .size(64.dp)
                .background(Brush.verticalGradient(listOf(c.accent, c.indigo)), CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            VynIconView(VynIcon.Phone, tint = c.onAccent, size = 30.dp)
        }
    }
}

/** Posição estável do PC no radar, derivada do `id`: mesmo PC, mesmo lugar, sem sobreposição fixa. */
private fun blipPolar(id: String): Pair<Float, Float> {
    val h = id.hashCode() and 0x7fffffff
    val angle = (h % 360).toFloat()
    val distance = 0.40f + 0.42f * (((h / 360) % 100) / 100f)
    return angle to distance
}

@Composable
private fun RadarBlip(pc: DiscoveredPc, radiusPx: Float, sweep: State<Float>, onSelect: (DiscoveredPc) -> Unit) {
    val c = VynTheme.colors
    val haptics = LocalHaptics.current
    val appear = remember { Animatable(0f) }
    val ring = remember { Animatable(0f) }
    LaunchedEffect(Unit) {
        haptics.confirm()
        appear.animateTo(1f, vynSpring())
    }
    LaunchedEffect(Unit) { ring.animateTo(1f, tween(1100)) }

    val (angle, distance) = remember(pc.id) { blipPolar(pc.id) }
    val rad = angle * (PI.toFloat() / 180f)
    val half = with(LocalDensity.current) { 28.dp.roundToPx() }
    val x = (radiusPx + radiusPx * distance * cos(rad)).toInt() - half
    val y = (radiusPx + radiusPx * distance * sin(rad)).toInt() - half
    val color = if (pc.freeSlots > 0 || pc.isManual) c.success else c.warning

    Box(
        Modifier
            .offset { IntOffset(x, y) }
            .size(56.dp)
            .graphicsLayer {
                scaleX = appear.value
                scaleY = appear.value
                // Acende quando a varredura acabou de passar e vai apagando até a próxima volta.
                val since = (sweep.value - angle + 360f) % 360f
                alpha = (1f - since / 260f).coerceIn(0.45f, 1f)
            }
            .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {
                haptics.tick()
                onSelect(pc)
            },
        contentAlignment = Alignment.Center,
    ) {
        Canvas(Modifier.fillMaxSize()) {
            val center = Offset(size.width / 2f, size.height / 2f)
            drawCircle(color.copy(alpha = 0.22f), size.minDimension / 2f * 0.62f, center)
            if (ring.value < 1f) {
                drawCircle(color.copy(alpha = (1f - ring.value) * 0.7f), size.minDimension / 2f * (0.3f + 0.7f * ring.value), center, style = Stroke(2.dp.toPx()))
            }
            drawCircle(color, 6.dp.toPx(), center)
            drawCircle(Color.White.copy(alpha = 0.55f), 2.2.dp.toPx(), center - Offset(1.5.dp.toPx(), 1.5.dp.toPx()))
        }
    }
}
