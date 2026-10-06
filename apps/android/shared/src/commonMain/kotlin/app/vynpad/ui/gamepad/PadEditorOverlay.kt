package app.vynpad.ui.gamepad

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.drag
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import app.vynpad.ui.theme.VynTheme

/**
 * A camada do editor de layout, sobre o controle desenhado: um dedo seleciona e arrasta uma peça
 * (inclusive as escondidas, tracejadas), com a grade do ímã visível. Não mexe no layout: avisa
 * quem chamou por [onSelect], [onMove] (centro em fração da tela) e [onMoveEnd].
 */
@Composable
fun PadEditorOverlay(
    layout: PadLayout,
    selected: PadElement?,
    snap: Boolean,
    onSelect: (PadElement?) -> Unit,
    onMove: (PadElement, x: Float, y: Float, aspect: Float) -> Unit,
    onMoveEnd: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = VynTheme.colors
    var size by remember { mutableStateOf(IntSize.Zero) }
    var dragging by remember { mutableStateOf(false) }
    val currentLayout by rememberUpdatedState(layout)
    val currentSize by rememberUpdatedState(size)
    val currentSelect by rememberUpdatedState(onSelect)
    val currentMove by rememberUpdatedState(onMove)
    val currentEnd by rememberUpdatedState(onMoveEnd)

    Canvas(
        modifier
            .fillMaxSize()
            .onSizeChanged { size = it }
            .pointerInput(Unit) {
                awaitEachGesture {
                    val down = awaitFirstDown()
                    val s = currentSize
                    if (s.width == 0 || s.height == 0) return@awaitEachGesture
                    val g = GamepadGeometry(s.width.toFloat(), s.height.toFloat(), currentLayout)
                    val element = g.elementAt(down.position.x, down.position.y)
                    currentSelect(element)
                    down.consume()
                    if (element == null) return@awaitEachGesture
                    // O dedo segura a peça onde a pegou: ela não pula para o centro do toque.
                    val spec = currentLayout.spec(element)
                    val grab = Offset(down.position.x - spec.x * s.width, down.position.y - spec.y * s.height)
                    dragging = true
                    drag(down.id) { change ->
                        change.consume()
                        currentMove(
                            element,
                            (change.position.x - grab.x) / s.width,
                            (change.position.y - grab.y) / s.height,
                            s.width.toFloat() / s.height,
                        )
                    }
                    dragging = false
                    currentEnd()
                }
            },
    ) {
        val g = GamepadGeometry(size.width.toFloat(), size.height.toFloat(), layout)
        if (snap) drawGrid(g, c.textTertiary.copy(alpha = if (dragging) 0.35f else 0.14f))

        val dash = PathEffect.dashPathEffect(floatArrayOf(10.dp.toPx(), 7.dp.toPx()))
        val corner = CornerRadius(14.dp.toPx())
        for (element in PadElement.entries) {
            val b = g.bounds(element)
            val isSelected = element == selected
            val hidden = !layout.spec(element).visible
            if (!isSelected && !hidden) continue
            val pad = 6.dp.toPx()
            val topLeft = Offset(b.left - pad, b.top - pad)
            val boxSize = Size(b.width + 2 * pad, b.height + 2 * pad)
            if (isSelected) {
                drawRoundRect(c.accent.copy(alpha = 0.10f), topLeft, boxSize, corner)
                drawRoundRect(c.accent, topLeft, boxSize, corner, style = Stroke(2.dp.toPx()))
            } else {
                drawRoundRect(Color.White.copy(alpha = 0.04f), topLeft, boxSize, corner)
                drawRoundRect(c.textTertiary, topLeft, boxSize, corner, style = Stroke(1.5.dp.toPx(), pathEffect = dash))
            }
        }
    }
}

/** Pontos de grade com o mesmo espaçamento em pixels nos dois eixos. */
private fun androidx.compose.ui.graphics.drawscope.DrawScope.drawGrid(g: GamepadGeometry, color: Color) {
    val step = PadEditing.GRID * g.height
    val r = 1.4.dp.toPx()
    var x = step
    while (x < g.width) {
        var y = step
        while (y < g.height) {
            drawCircle(color, r, Offset(x, y))
            y += step
        }
        x += step
    }
}
