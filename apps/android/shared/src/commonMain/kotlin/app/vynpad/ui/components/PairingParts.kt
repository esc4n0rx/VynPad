package app.vynpad.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathMeasure
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import app.vynpad.ui.haptics.LocalHaptics
import app.vynpad.ui.theme.VynTheme
import app.vynpad.ui.theme.vynSpring

/** Moldura de leitura do QR: cantos de accent e uma linha de varredura que sobe e desce. */
@Composable
fun ScanFrame(modifier: Modifier = Modifier, tint: Color = VynTheme.colors.accent) {
    val transition = rememberInfiniteTransition()
    val line = transition.animateFloat(0.08f, 0.92f, infiniteRepeatable(tween(1900, easing = LinearEasing), RepeatMode.Reverse))
    val breathe = transition.animateFloat(0.55f, 1f, infiniteRepeatable(tween(1100), RepeatMode.Reverse))
    Canvas(modifier.fillMaxSize()) {
        val inset = 28.dp.toPx()
        val len = 36.dp.toPx()
        val w = 4.dp.toPx()
        val left = inset
        val top = inset
        val right = size.width - inset
        val bottom = size.height - inset
        val color = tint.copy(alpha = breathe.value)
        fun corner(x: Float, y: Float, dx: Float, dy: Float) {
            val p = Path().apply {
                moveTo(x, y + dy * len)
                lineTo(x, y)
                lineTo(x + dx * len, y)
            }
            drawPath(p, color, style = Stroke(w, cap = StrokeCap.Round, join = StrokeJoin.Round))
        }
        corner(left, top, 1f, 1f)
        corner(right, top, -1f, 1f)
        corner(left, bottom, 1f, -1f)
        corner(right, bottom, -1f, -1f)
        val y = top + (bottom - top) * line.value
        drawLine(
            Brush.horizontalGradient(listOf(Color.Transparent, tint, Color.Transparent), left, right),
            Offset(left, y),
            Offset(right, y),
            3.dp.toPx(),
            StrokeCap.Round,
        )
    }
}

/** Seis células do PIN. O erro faz as células tremerem (e o chamador limpa o PIN). */
@Composable
fun PinCells(pin: String, length: Int, errorTick: Int, modifier: Modifier = Modifier) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    val shake = remember { Animatable(0f) }
    LaunchedEffect(errorTick) {
        if (errorTick == 0) return@LaunchedEffect
        for (target in listOf(-14f, 12f, -9f, 6f, -3f, 0f)) shake.animateTo(target, tween(55))
    }
    Row(
        modifier.offset { IntOffset(shake.value.dp.roundToPx(), 0) },
        horizontalArrangement = Arrangement.spacedBy(10.dp, Alignment.CenterHorizontally),
    ) {
        repeat(length) { i ->
            val filled = i < pin.length
            val active = i == pin.length
            val border by animateFloatAsState(if (active) 1f else 0f, vynSpring())
            val pop by animateFloatAsState(if (filled) 1f else 0.9f, vynSpring())
            Box(
                Modifier
                    .width(46.dp)
                    .height(58.dp)
                    .scale(pop)
                    .clip(RoundedCornerShape(14.dp))
                    .background(if (filled) c.accentSoft else c.control)
                    .border(2.dp, c.accent.copy(alpha = border), RoundedCornerShape(14.dp)),
                contentAlignment = Alignment.Center,
            ) {
                if (filled) VynText(pin[i].toString(), style = t.title)
            }
        }
    }
}

/** Teclado numérico no estilo do app: teclas redondas que encolhem com mola e dão haptic. */
@Composable
fun NumericKeypad(onDigit: (Char) -> Unit, onDelete: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true) {
    val rows = listOf("123", "456", "789", " 0<")
    Column(modifier, verticalArrangement = Arrangement.spacedBy(12.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        rows.forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(18.dp)) {
                row.forEach { key ->
                    when (key) {
                        ' ' -> Box(Modifier.size(KeySize))
                        '<' -> Key(label = null, enabled = enabled, onClick = onDelete)
                        else -> Key(label = key.toString(), enabled = enabled) { onDigit(key) }
                    }
                }
            }
        }
    }
}

private val KeySize = 68.dp

@Composable
private fun Key(label: String?, enabled: Boolean, onClick: () -> Unit) {
    val c = VynTheme.colors
    val haptics = LocalHaptics.current
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val scale by animateFloatAsState(if (pressed) 0.88f else 1f, vynSpring(0.001f))
    Box(
        Modifier
            .size(KeySize)
            .scale(scale)
            .clip(CircleShape)
            .background(if (pressed) c.controlHover else c.control)
            .clickable(interactionSource = interaction, indication = null, enabled = enabled) {
                if (label == null) haptics.tick() else haptics.press()
                onClick()
            },
        contentAlignment = Alignment.Center,
    ) {
        if (label != null) {
            VynText(label, style = VynTheme.typography.title.copy(textAlign = TextAlign.Center))
        } else {
            Canvas(Modifier.size(26.dp)) {
                val u = size.minDimension / 24f
                val stroke = Stroke(1.75f * u, cap = StrokeCap.Round, join = StrokeJoin.Round)
                val box = Path().apply {
                    moveTo(9f * u, 5f * u)
                    lineTo(21f * u, 5f * u)
                    lineTo(21f * u, 19f * u)
                    lineTo(9f * u, 19f * u)
                    lineTo(2f * u, 12f * u)
                    close()
                }
                drawPath(box, c.textPrimary, style = stroke)
                drawLine(c.textPrimary, Offset(12f * u, 9f * u), Offset(17f * u, 15f * u), stroke.width, StrokeCap.Round)
                drawLine(c.textPrimary, Offset(17f * u, 9f * u), Offset(12f * u, 15f * u), stroke.width, StrokeCap.Round)
            }
        }
    }
}

/** Anéis de accent que saem de um ponto central: "conectando". */
@Composable
fun PulseRings(modifier: Modifier = Modifier, color: Color = VynTheme.colors.accent) {
    val transition = rememberInfiniteTransition()
    val phase = transition.animateFloat(0f, 1f, infiniteRepeatable(tween(1600, easing = LinearEasing)))
    Canvas(modifier) {
        val center = Offset(size.width / 2f, size.height / 2f)
        val max = size.minDimension / 2f
        listOf(0f, 0.5f).forEach { shift ->
            val p = (phase.value + shift) % 1f
            drawCircle(color.copy(alpha = (1f - p) * 0.6f), max * (0.25f + 0.75f * p), center, style = Stroke(2.5.dp.toPx()))
        }
        drawCircle(color, max * 0.16f, center)
    }
}

/** Selo de sucesso: o disco cresce com mola, uma onda sai dele e o check é desenhado traço a traço. */
@Composable
fun SuccessBadge(modifier: Modifier = Modifier, color: Color = VynTheme.colors.success) {
    val disc = remember { Animatable(0f) }
    val wave = remember { Animatable(0f) }
    val check = remember { Animatable(0f) }
    LaunchedEffect(Unit) {
        disc.animateTo(1f, vynSpring())
    }
    LaunchedEffect(Unit) { wave.animateTo(1f, tween(900)) }
    LaunchedEffect(Unit) {
        kotlinx.coroutines.delay(200)
        check.animateTo(1f, tween(420))
    }
    Canvas(modifier) {
        val center = Offset(size.width / 2f, size.height / 2f)
        val r = size.minDimension / 2f
        if (wave.value < 1f) {
            drawCircle(color.copy(alpha = (1f - wave.value) * 0.55f), r * (0.7f + 0.3f * wave.value), center, style = Stroke(3.dp.toPx()))
        }
        drawCircle(color.copy(alpha = 0.18f), r * 0.78f * disc.value, center)
        drawCircle(color, r * 0.56f * disc.value, center)
        val u = r * 0.56f / 12f
        val path = Path().apply {
            moveTo(center.x - 6.5f * u, center.y + 0.5f * u)
            lineTo(center.x - 1.8f * u, center.y + 5f * u)
            lineTo(center.x + 6.8f * u, center.y - 4.6f * u)
        }
        val measure = PathMeasure().apply { setPath(path, false) }
        val partial = Path()
        measure.getSegment(0f, measure.length * check.value, partial, true)
        drawPath(partial, Color.White, style = Stroke(4.dp.toPx(), cap = StrokeCap.Round, join = StrokeJoin.Round))
    }
}

/** Linha de texto de erro/ajuda centralizada. */
@Composable
fun HintText(text: String, color: Color, modifier: Modifier = Modifier) {
    VynText(
        text,
        style = VynTheme.typography.body.copy(textAlign = TextAlign.Center),
        color = color,
        modifier = modifier.fillMaxWidth(),
    )
}
