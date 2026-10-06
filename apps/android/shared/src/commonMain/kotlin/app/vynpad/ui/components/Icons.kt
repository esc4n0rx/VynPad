package app.vynpad.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** Ícones desenhados em Canvas, traço de 1,75 como o lucide do client Windows. */
enum class VynIcon { Connect, Settings, Gamepad, Back, Phone }

@Composable
fun VynIconView(icon: VynIcon, tint: Color, modifier: Modifier = Modifier, size: Dp = 24.dp) {
    Canvas(modifier.size(size)) {
        val u = this.size.minDimension / 24f
        val stroke = Stroke(width = 1.75f * u, cap = StrokeCap.Round, join = StrokeJoin.Round)
        when (icon) {
            VynIcon.Connect -> connect(tint, stroke, u)
            VynIcon.Settings -> settings(tint, stroke, u)
            VynIcon.Gamepad -> gamepad(tint, stroke, u)
            VynIcon.Back -> back(tint, stroke, u)
            VynIcon.Phone -> phone(tint, stroke, u)
        }
    }
}

private fun DrawScope.connect(tint: Color, stroke: Stroke, u: Float) {
    // Ondas de sinal em torno de um ponto.
    drawCircle(tint, radius = 1.6f * u, center = Offset(12f * u, 18f * u))
    listOf(5f, 9f, 13f).forEach { r ->
        val p = Path().apply {
            arcTo(
                rect = androidx.compose.ui.geometry.Rect(Offset(12f * u, 18f * u) - Offset(r * u, r * u), Size(2 * r * u, 2 * r * u)),
                startAngleDegrees = 225f,
                sweepAngleDegrees = 90f,
                forceMoveTo = true,
            )
        }
        drawPath(p, tint, style = stroke)
    }
}

private fun DrawScope.settings(tint: Color, stroke: Stroke, u: Float) {
    // Três sliders.
    val rows = listOf(6f to 8f, 12f to 16f, 18f to 10f)
    rows.forEach { (y, knob) ->
        drawLine(tint, Offset(4f * u, y * u), Offset(20f * u, y * u), stroke.width, StrokeCap.Round)
        drawCircle(tint, radius = 2.4f * u, center = Offset(knob * u, y * u))
    }
}

private fun DrawScope.gamepad(tint: Color, stroke: Stroke, u: Float) {
    val body = Path().apply {
        moveTo(7f * u, 7f * u)
        lineTo(17f * u, 7f * u)
        cubicTo(20.5f * u, 7f * u, 22f * u, 10f * u, 22f * u, 14f * u)
        cubicTo(22f * u, 17f * u, 20.5f * u, 18f * u, 19f * u, 17f * u)
        lineTo(16f * u, 15f * u)
        lineTo(8f * u, 15f * u)
        lineTo(5f * u, 17f * u)
        cubicTo(3.5f * u, 18f * u, 2f * u, 17f * u, 2f * u, 14f * u)
        cubicTo(2f * u, 10f * u, 3.5f * u, 7f * u, 7f * u, 7f * u)
        close()
    }
    drawPath(body, tint, style = stroke)
    drawLine(tint, Offset(6f * u, 11f * u), Offset(10f * u, 11f * u), stroke.width, StrokeCap.Round)
    drawLine(tint, Offset(8f * u, 9f * u), Offset(8f * u, 13f * u), stroke.width, StrokeCap.Round)
    drawCircle(tint, radius = 0.9f * u, center = Offset(15f * u, 10.5f * u))
    drawCircle(tint, radius = 0.9f * u, center = Offset(18f * u, 12.5f * u))
}

private fun DrawScope.back(tint: Color, stroke: Stroke, u: Float) {
    val p = Path().apply {
        moveTo(15f * u, 5f * u)
        lineTo(8f * u, 12f * u)
        lineTo(15f * u, 19f * u)
    }
    drawPath(p, tint, style = stroke)
}

private fun DrawScope.phone(tint: Color, stroke: Stroke, u: Float) {
    drawRoundRect(
        tint,
        topLeft = Offset(7f * u, 2.5f * u),
        size = Size(10f * u, 19f * u),
        cornerRadius = androidx.compose.ui.geometry.CornerRadius(2.5f * u),
        style = stroke,
    )
    drawLine(tint, Offset(11f * u, 18.5f * u), Offset(13f * u, 18.5f * u), stroke.width, StrokeCap.Round)
}
