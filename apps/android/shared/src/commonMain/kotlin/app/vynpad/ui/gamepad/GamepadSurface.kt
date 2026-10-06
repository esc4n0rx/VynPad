package app.vynpad.ui.gamepad

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.Paint
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.input.pointer.changedToDown
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import app.vynpad.input.GamepadState
import app.vynpad.input.PadButtons
import app.vynpad.skin.SkinBitmap
import app.vynpad.ui.theme.VynColors
import app.vynpad.ui.theme.VynTheme
import app.vynpad.ui.theme.vynSpring
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

/** Botões que acendem com uma mola cada: cada máscara tem o seu progresso 0..1. */
private val AnimatedMasks = intArrayOf(
    PadButtons.A, PadButtons.B, PadButtons.X, PadButtons.Y,
    PadButtons.LEFT_SHOULDER, PadButtons.RIGHT_SHOULDER,
    PadButtons.DPAD_UP, PadButtons.DPAD_DOWN, PadButtons.DPAD_LEFT, PadButtons.DPAD_RIGHT,
    PadButtons.BACK, PadButtons.START, PadButtons.GUIDE,
    PadButtons.LEFT_THUMB, PadButtons.RIGHT_THUMB,
)

private class PressAnimators {
    val progress = AnimatedMasks.associateWith { Animatable(0f) }
    private val targets = HashMap<Int, Float>()

    fun update(buttons: Int, scope: CoroutineScope) {
        for ((mask, anim) in progress) {
            val target = if (buttons and mask != 0) 1f else 0f
            if (targets[mask] != target) {
                targets[mask] = target
                scope.launch { anim.animateTo(target, vynSpring(0.001f)) }
            }
        }
    }

    operator fun get(mask: Int): Float = progress[mask]?.value ?: 0f
}

/** Um analógico animado: o polegar volta ao centro com mola e a base some ao soltar. */
private class StickAnimator {
    val knob = Animatable(Offset.Zero, Offset.VectorConverter)
    val active = Animatable(0f)
    var origin by mutableStateOf(Offset.Zero)
}

/**
 * O controle na tela: desenha tudo num `Canvas` e recebe os toques de todos os dedos, que o
 * [TouchRouter] converte em [GamepadState]. Nada aqui recompõe por toque: os estados mudam e só
 * o desenho é refeito.
 */
@Composable
fun GamepadSurface(
    state: GamepadState,
    accent: Color,
    feedback: PadFeedback,
    layout: PadLayout,
    background: SkinBitmap? = null,
    modifier: Modifier = Modifier,
    /** Falso com o menu aberto: solta tudo e ignora os toques. */
    enabled: Boolean = true,
) {
    val colors = VynTheme.colors
    val typography = VynTheme.typography
    val measurer = rememberTextMeasurer()
    val labels = remember(typography, measurer) {
        listOf("A", "B", "X", "Y", "LB", "RB", "LT", "RT").associateWith {
            measurer.measure(AnnotatedString(it), typography.bodyStrong)
        }
    }

    val visuals = remember { PadVisuals() }
    var size by remember { mutableStateOf(IntSize.Zero) }
    // A geometria acompanha o layout: editar uma peça move o desenho e a área de toque juntos.
    val geometry = remember(size, layout) {
        if (size.width > 0 && size.height > 0) GamepadGeometry(size.width.toFloat(), size.height.toFloat(), layout) else null
    }
    val currentGeometry by rememberUpdatedState(geometry)
    val currentFeedback by rememberUpdatedState(feedback)
    val router = remember(state) {
        TouchRouter(
            geometry = { currentGeometry ?: GamepadGeometry(1f, 1f) },
            state = state,
            visuals = visuals,
            feedback = object : PadFeedback {
                override fun press() = currentFeedback.press()
                override fun slide() = currentFeedback.slide()
                override fun detent() = currentFeedback.detent()
            },
        )
    }

    val currentEnabled by rememberUpdatedState(enabled)
    LaunchedEffect(enabled) { if (!enabled) router.cancelAll() }

    val presses = remember { PressAnimators() }
    val leftAnim = remember { StickAnimator() }
    val rightAnim = remember { StickAnimator() }

    LaunchedEffect(visuals) {
        snapshotFlow { visuals.buttons }.collect { presses.update(it, this) }
    }
    LaunchedEffect(visuals) { followStick(this, leftAnim) { visuals.leftStick } }
    LaunchedEffect(visuals) { followStick(this, rightAnim) { visuals.rightStick } }
    DisposableEffect(router) {
        onDispose {
            router.cancelAll()
            state.reset()
        }
    }

    Canvas(
        modifier
            .fillMaxSize()
            .onSizeChanged { size = it }
            .pointerInput(router) {
                awaitPointerEventScope {
                    while (true) {
                        val event = awaitPointerEvent()
                        if (!currentEnabled) continue
                        for (change in event.changes) {
                            val id = change.id.value
                            val p = change.position
                            when {
                                change.changedToDown() -> router.down(id, p.x, p.y, change.uptimeMillis)
                                change.changedToUp() -> router.up(id, change.uptimeMillis)
                                change.pressed -> router.move(id, p.x, p.y)
                            }
                            if (change.pressed || change.changedToUp()) change.consume()
                        }
                    }
                }
            },
    ) {
        val g = geometry ?: return@Canvas
        background?.let { drawSkinBackground(it.image, layout.skin.veil) }
        drawPad(g, colors, accent, presses, visuals, leftAnim, rightAnim, labels, measurer, layout.skin.padOpacity)
    }
}

/** Sincroniza o desenho de um analógico com o estado: segue o dedo e volta ao centro com mola. */
private suspend fun followStick(scope: CoroutineScope, anim: StickAnimator, read: () -> StickVisual) {
    snapshotFlow(read).collect { visual ->
        val origin = visual.origin
        if (origin != null) {
            anim.origin = origin
            scope.launch { anim.active.animateTo(1f, tween(90)) }
            anim.knob.snapTo(visual.knob)
        } else {
            scope.launch { anim.active.animateTo(0f, tween(220)) }
            scope.launch { anim.knob.animateTo(Offset.Zero, vynSpring()) }
        }
    }
}

private fun DrawScope.drawPad(
    g: GamepadGeometry,
    c: VynColors,
    accent: Color,
    presses: PressAnimators,
    visuals: PadVisuals,
    left: StickAnimator,
    right: StickAnimator,
    labels: Map<String, TextLayoutResult>,
    measurer: TextMeasurer,
    padOpacity: Float,
) {
    // Cada peça some se o usuário a escondeu e usa a opacidade que ele escolheu.
    fun part(element: PadElement, draw: DrawScope.() -> Unit) {
        val spec = g.layout.spec(element)
        if (spec.visible) layer(spec.opacity * padOpacity, g.bounds(element), draw)
    }

    // ---- Gatilhos e ombros
    part(PadElement.LT) { drawTrigger(g.lt, visuals.lt, c, accent, labels["LT"]) }
    part(PadElement.RT) { drawTrigger(g.rt, visuals.rt, c, accent, labels["RT"]) }
    part(PadElement.LB) { drawShoulder(g.lb, presses[PadButtons.LEFT_SHOULDER], c, accent, labels["LB"]) }
    part(PadElement.RB) { drawShoulder(g.rb, presses[PadButtons.RIGHT_SHOULDER], c, accent, labels["RB"]) }

    // ---- Centro
    part(PadElement.Back) { drawCenterButton(g.back, presses[PadButtons.BACK], c, accent, CenterGlyph.Back) }
    part(PadElement.Guide) { drawCenterButton(g.guide, presses[PadButtons.GUIDE], c, accent, CenterGlyph.Guide) }
    part(PadElement.Start) { drawCenterButton(g.start, presses[PadButtons.START], c, accent, CenterGlyph.Start) }

    // ---- Direcional e ABXY
    part(PadElement.Dpad) { drawDpad(g.dpad, presses, c, accent) }
    part(PadElement.Face) {
        for (button in g.faceButtons) {
            drawRound(button.shape, presses[button.mask], c, accent, labels[button.label])
        }
    }

    // ---- Analógicos
    part(PadElement.LeftStick) { drawStick(g.leftStickZone, g, true, left, presses[PadButtons.LEFT_THUMB], c, accent) }
    part(PadElement.RightStick) { drawStick(g.rightStickZone, g, false, right, presses[PadButtons.RIGHT_THUMB], c, accent) }
}

/** A imagem do jogo cobrindo a tela (corte central, sem distorcer) e o véu escuro por cima. */
private fun DrawScope.drawSkinBackground(image: ImageBitmap, veil: Float) {
    val scale = maxOf(size.width / image.width, size.height / image.height)
    val srcW = (size.width / scale).toInt().coerceIn(1, image.width)
    val srcH = (size.height / scale).toInt().coerceIn(1, image.height)
    drawImage(
        image,
        srcOffset = IntOffset((image.width - srcW) / 2, (image.height - srcH) / 2),
        srcSize = IntSize(srcW, srcH),
        dstSize = IntSize(size.width.toInt(), size.height.toInt()),
        filterQuality = FilterQuality.Low,
    )
    if (veil > 0f) drawRect(Color.Black.copy(alpha = veil))
}

/**
 * Desenha [draw] com a opacidade da peça. Com opacidade total não há custo; abaixo disso a peça
 * vai para uma camada própria (halos e contornos se fundem antes de esmaecer, sem manchas).
 */
private fun DrawScope.layer(opacity: Float, bounds: Box, draw: DrawScope.() -> Unit) {
    if (opacity >= 0.99f) {
        draw()
        return
    }
    val margin = maxOf(bounds.width, bounds.height) * 0.4f
    val rect = Rect(bounds.left - margin, bounds.top - margin, bounds.right + margin, bounds.bottom + margin)
    val paint = Paint().apply { alpha = opacity }
    drawIntoCanvas { canvas ->
        canvas.saveLayer(rect, paint)
        draw()
        canvas.restore()
    }
}

private fun padFill(c: VynColors, accent: Color, pressed: Float, top: Float, height: Float): Brush {
    val base = lerp(c.padTop, accent, pressed)
    val bottom = lerp(c.padBottom, accent.copy(alpha = 0.78f), pressed)
    return Brush.verticalGradient(listOf(base, bottom), startY = top, endY = top + height)
}

/** Disco de vidro: acende na cor do slot com um halo e encolhe um pouco ao ser pressionado. */
private fun DrawScope.drawRound(shape: Circle, pressed: Float, c: VynColors, accent: Color, label: TextLayoutResult?) {
    val center = Offset(shape.cx, shape.cy)
    if (pressed > 0.01f) {
        drawCircle(accent.copy(alpha = 0.32f * pressed), shape.r * (1.15f + 0.45f * pressed), center)
    }
    scale(1f - 0.07f * pressed, center) {
        drawCircle(padFill(c, accent, pressed, shape.cy - shape.r, shape.r * 2), shape.r, center)
        drawCircle(c.padEdge, shape.r, center, style = Stroke(1.5.dp.toPx()))
        label?.let {
            val color = lerp(c.padGlyph, Color.White, pressed)
            drawText(it, color, Offset(center.x - it.size.width / 2f, center.y - it.size.height / 2f))
        }
    }
}

private fun DrawScope.drawShoulder(box: Box, pressed: Float, c: VynColors, accent: Color, label: TextLayoutResult?) {
    val topLeft = Offset(box.left, box.top)
    val size = Size(box.width, box.height)
    val radius = CornerRadius(box.height * 0.45f)
    if (pressed > 0.01f) {
        drawRoundRect(accent.copy(alpha = 0.28f * pressed), topLeft - Offset(8f, 8f), Size(size.width + 16f, size.height + 16f), radius)
    }
    drawRoundRect(padFill(c, accent, pressed, box.top, box.height), topLeft, size, radius)
    drawRoundRect(c.padEdge, topLeft, size, radius, style = Stroke(1.5.dp.toPx()))
    label?.let {
        val color = lerp(c.padGlyph, Color.White, pressed)
        drawText(it, color, Offset(box.left + (box.width - it.size.width) / 2f, box.top + (box.height - it.size.height) / 2f))
    }
}

/** Gatilho analógico: o preenchimento sobe com a pressão e há uma trilha marcando o curso. */
private fun DrawScope.drawTrigger(box: Box, value: Float, c: VynColors, accent: Color, label: TextLayoutResult?) {
    val topLeft = Offset(box.left, box.top)
    val size = Size(box.width, box.height)
    val radius = CornerRadius(box.height * 0.45f)
    drawRoundRect(c.padWell, topLeft, size, radius)
    if (value > 0.01f) {
        val fill = box.height * value.coerceIn(0f, 1f)
        drawRoundRect(accent.copy(alpha = 0.35f + 0.55f * value), topLeft, Size(size.width, fill.coerceAtLeast(box.height * 0.2f)), radius)
        drawRoundRect(accent.copy(alpha = 0.25f * value), topLeft - Offset(6f, 6f), Size(size.width + 12f, size.height + 12f), radius)
    }
    drawRoundRect(c.padEdge, topLeft, size, radius, style = Stroke(1.5.dp.toPx()))
    label?.let {
        val color = lerp(c.padGlyph, Color.White, value)
        drawText(it, color, Offset(box.left + (box.width - it.size.width) / 2f, box.top + (box.height - it.size.height) / 2f))
    }
}

private enum class CenterGlyph { Back, Guide, Start }

private fun DrawScope.drawCenterButton(button: RoundButton, pressed: Float, c: VynColors, accent: Color, glyph: CenterGlyph) {
    val s = button.shape
    val center = Offset(s.cx, s.cy)
    if (pressed > 0.01f) drawCircle(accent.copy(alpha = 0.3f * pressed), s.r * (1.2f + 0.4f * pressed), center)
    scale(1f - 0.08f * pressed, center) {
        drawCircle(padFill(c, accent, pressed, s.cy - s.r, s.r * 2), s.r, center)
        drawCircle(c.padEdge, s.r, center, style = Stroke(1.5.dp.toPx()))
        val ink = lerp(c.padGlyph, Color.White, pressed)
        val stroke = Stroke(2.dp.toPx(), cap = androidx.compose.ui.graphics.StrokeCap.Round)
        val u = s.r * 0.5f
        when (glyph) {
            CenterGlyph.Start -> repeat(3) { i ->
                val y = s.cy + (i - 1) * u * 0.6f
                drawLine(ink, Offset(s.cx - u * 0.7f, y), Offset(s.cx + u * 0.7f, y), stroke.width, stroke.cap)
            }

            CenterGlyph.Back -> {
                drawRoundRect(ink, Offset(s.cx - u * 0.8f, s.cy - u * 0.35f), Size(u * 1.1f, u * 0.9f), CornerRadius(u * 0.2f), style = stroke)
                drawRoundRect(ink, Offset(s.cx - u * 0.3f, s.cy - u * 0.65f), Size(u * 1.1f, u * 0.9f), CornerRadius(u * 0.2f), style = stroke)
            }

            CenterGlyph.Guide -> {
                drawCircle(ink, u * 0.95f, center, style = stroke)
                drawCircle(ink, u * 0.32f, center)
            }
        }
    }
}

/**
 * Direcional: quatro botões separados em cruz (sem se tocarem), cada um acende sozinho na cor
 * do slot. As diagonais acendem os dois vizinhos.
 */
private fun DrawScope.drawDpad(zone: Circle, presses: PressAnimators, c: VynColors, accent: Color) {
    val center = Offset(zone.cx, zone.cy)
    drawCircle(c.padWell, zone.r, center)
    val arm = zone.r * 0.40f
    val width = zone.r * 0.46f
    val gap = zone.r * 0.30f
    val corner = CornerRadius(width * 0.30f)
    fun arm(mask: Int, topLeft: Offset, size: Size) {
        val p = presses[mask]
        if (p > 0.01f) {
            drawRoundRect(accent.copy(alpha = 0.30f * p), topLeft - Offset(6f, 6f), Size(size.width + 12f, size.height + 12f), corner)
        }
        drawRoundRect(padFill(c, accent, p, topLeft.y, size.height), topLeft, size, corner)
        drawRoundRect(c.padEdge, topLeft, size, corner, style = Stroke(1.2.dp.toPx()))
    }
    arm(PadButtons.DPAD_UP, Offset(zone.cx - width / 2, zone.cy - gap - arm), Size(width, arm))
    arm(PadButtons.DPAD_DOWN, Offset(zone.cx - width / 2, zone.cy + gap), Size(width, arm))
    arm(PadButtons.DPAD_LEFT, Offset(zone.cx - gap - arm, zone.cy - width / 2), Size(arm, width))
    arm(PadButtons.DPAD_RIGHT, Offset(zone.cx + gap, zone.cy - width / 2), Size(arm, width))
    // Um ponto discreto marca o centro.
    drawCircle(c.padEdge, zone.r * 0.07f, center)
}

/**
 * Analógico flutuante: em repouso é só uma base apagada na zona; ao tocar, a base nasce onde o
 * dedo pousou e o polegar acompanha o dedo; ao soltar, o polegar volta ao centro com mola.
 */
private fun DrawScope.drawStick(zone: Circle, g: GamepadGeometry, left: Boolean, anim: StickAnimator, clicked: Float, c: VynColors, accent: Color) {
    val rest = Offset(zone.cx, zone.cy)
    val travel = g.travelOf(left)
    val active = anim.active.value
    // Base de repouso (some quando o analógico está em uso).
    drawCircle(c.padWell.copy(alpha = c.padWell.alpha * (1f - 0.6f * active)), travel * 1.25f, rest)
    drawCircle(c.padEdge.copy(alpha = c.padEdge.alpha * (1f - active)), travel * 1.25f, rest, style = Stroke(1.5.dp.toPx()))
    if (active < 0.99f) drawKnob(rest + anim.knob.value * (1f - active), g.knobOf(left), c, accent, clicked, 0f)

    if (active > 0.01f) {
        val origin = anim.origin
        drawCircle(accent.copy(alpha = 0.10f * active), travel * 1.25f, origin)
        drawCircle(accent.copy(alpha = 0.55f * active), travel * 1.25f, origin, style = Stroke(2.dp.toPx()))
        drawKnob(origin + anim.knob.value, g.knobOf(left), c, accent, clicked, active)
    }
}

private fun DrawScope.drawKnob(at: Offset, r: Float, c: VynColors, accent: Color, clicked: Float, active: Float) {
    if (active > 0.01f || clicked > 0.01f) {
        drawCircle(accent.copy(alpha = 0.28f * maxOf(active, clicked)), r * 1.5f, at)
    }
    scale(1f - 0.08f * clicked, at) {
        drawCircle(padFill(c, accent, clicked, at.y - r, r * 2), r, at)
        drawCircle(c.padEdge, r, at, style = Stroke(1.5.dp.toPx()))
        drawCircle(accent.copy(alpha = 0.18f + 0.55f * active), r * 0.55f, at, style = Stroke(2.dp.toPx()))
    }
}
