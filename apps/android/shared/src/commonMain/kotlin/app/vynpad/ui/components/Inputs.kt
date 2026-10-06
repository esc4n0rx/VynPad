package app.vynpad.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.drag
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import app.vynpad.ui.haptics.LocalHaptics
import app.vynpad.ui.theme.VynRadius
import app.vynpad.ui.theme.VynTheme

/**
 * Controle deslizante no estilo do app: trilho fino, polegar branco e o trecho preenchido na
 * cor de destaque. Toque e arraste em qualquer ponto; [onValueChangeFinished] sai ao soltar.
 */
@Composable
fun VynSlider(
    value: Float,
    onValueChange: (Float) -> Unit,
    valueRange: ClosedFloatingPointRange<Float>,
    modifier: Modifier = Modifier,
    onValueChangeFinished: () -> Unit = {},
) {
    val c = VynTheme.colors
    val currentChange by rememberUpdatedState(onValueChange)
    val currentFinished by rememberUpdatedState(onValueChangeFinished)
    val range by rememberUpdatedState(valueRange)
    val fraction = ((value - valueRange.start) / (valueRange.endInclusive - valueRange.start)).coerceIn(0f, 1f)
    Canvas(
        modifier
            .fillMaxWidth()
            .height(32.dp)
            .pointerInput(Unit) {
                val thumb = 12.dp.toPx()
                fun valueAt(x: Float): Float {
                    val f = ((x - thumb) / (size.width - 2 * thumb)).coerceIn(0f, 1f)
                    return range.start + f * (range.endInclusive - range.start)
                }
                awaitEachGesture {
                    val down = awaitFirstDown()
                    down.consume()
                    currentChange(valueAt(down.position.x))
                    drag(down.id) {
                        it.consume()
                        currentChange(valueAt(it.position.x))
                    }
                    currentFinished()
                }
            },
    ) {
        val thumb = 12.dp.toPx()
        val track = 4.dp.toPx()
        val y = size.height / 2
        val x = thumb + (size.width - 2 * thumb) * fraction
        drawRoundRect(c.controlHover, Offset(thumb, y - track / 2), Size(size.width - 2 * thumb, track), androidx.compose.ui.geometry.CornerRadius(track))
        drawRoundRect(c.accent, Offset(thumb, y - track / 2), Size(x - thumb, track), androidx.compose.ui.geometry.CornerRadius(track))
        drawCircle(Color.Black.copy(alpha = 0.18f), thumb + 1.dp.toPx(), Offset(x, y + 1.dp.toPx()))
        drawCircle(Color.White, thumb, Offset(x, y))
    }
}

/** Campo de texto de uma linha, com o visual dos campos do app. */
@Composable
fun VynTextField(value: String, onValueChange: (String) -> Unit, modifier: Modifier = Modifier, placeholder: String = "", onDone: () -> Unit = {}) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        singleLine = true,
        textStyle = t.body.copy(color = c.textPrimary),
        cursorBrush = SolidColor(c.accent),
        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
        keyboardActions = KeyboardActions(onDone = { onDone() }),
        modifier = modifier.fillMaxWidth(),
        decorationBox = { inner ->
            Box(
                Modifier
                    .fillMaxWidth()
                    .background(c.control, RoundedCornerShape(VynRadius.lg))
                    .padding(horizontal = 14.dp, vertical = 11.dp),
            ) {
                if (value.isEmpty()) VynText(placeholder, style = t.body, color = c.textTertiary)
                inner()
            }
        },
    )
}

/** Pílula de seleção (um item de uma lista curta). */
@Composable
fun VynChip(text: String, selected: Boolean, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val c = VynTheme.colors
    val haptics = LocalHaptics.current
    Box(
        modifier
            .clip(CircleShape)
            .background(if (selected) c.accent else c.control)
            .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {
                if (!selected) {
                    haptics.tick()
                    onClick()
                }
            }
            .padding(horizontal = 14.dp, vertical = 8.dp),
        contentAlignment = Alignment.Center,
    ) {
        VynText(
            text,
            style = VynTheme.typography.bodyStrong,
            color = if (selected) c.onAccent else c.textPrimary,
            maxLines = 1,
        )
    }
}
