package app.vynpad.ui.gamepad

import app.vynpad.input.PadButtons
import kotlin.math.abs
import kotlin.math.hypot

/** Círculo em pixels. */
data class Circle(val cx: Float, val cy: Float, val r: Float) {
    fun contains(x: Float, y: Float, scale: Float = 1f): Boolean = hypot(x - cx, y - cy) <= r * scale
}

/** Retângulo em pixels. */
data class Box(val left: Float, val top: Float, val right: Float, val bottom: Float) {
    val width get() = right - left
    val height get() = bottom - top

    fun contains(x: Float, y: Float, slop: Float = 0f): Boolean =
        x >= left - slop && x <= right + slop && y >= top - slop && y <= bottom + slop
}

/** Botão redondo do controle: a máscara que ele aciona e o rótulo desenhado nele. */
data class RoundButton(val mask: Int, val label: String, val shape: Circle)

/** Onde um toque caiu. */
sealed interface Hit {
    /** Botão redondo, ombro ou botão do centro. */
    data class Button(val mask: Int) : Hit

    /** Direcional: `mask` pode ser 0 (centro morto), uma direção ou uma diagonal. */
    data class Dpad(val mask: Int) : Hit

    data class Stick(val left: Boolean) : Hit

    data class Trigger(val left: Boolean) : Hit
}

/**
 * O layout do controle para uma tela de `width` × `height` pixels, em paisagem: tudo em
 * frações da altura e da largura, então vale de um celular de 5" a um tablet de 11".
 * As peças ficam onde o [layout] manda (o padrão reproduz o desenho original).
 * É código puro (sem Compose) para ser testado sem aparelho.
 */
class GamepadGeometry(val width: Float, val height: Float, val layout: PadLayout = PadLayouts.default()) {
    private val h = height

    private fun spec(element: PadElement) = layout.spec(element)

    private fun visible(element: PadElement) = spec(element).visible

    private fun circle(element: PadElement, radius: Float): Circle {
        val s = spec(element)
        return Circle(s.x * width, s.y * h, radius * s.scale)
    }

    // ---- Analógicos
    val leftStickZone = circle(PadElement.LeftStick, 0.25f * h)
    val rightStickZone = circle(PadElement.RightStick, 0.23f * h)

    /** Curso do analógico com a peça no tamanho original (a distância do centro à deflexão máxima). */
    val stickTravel = 0.11f * h

    /** Raio do polegar (cabeça) desenhado no analógico, no tamanho original. */
    val stickKnobRadius = 0.085f * h

    /** Curso deste analógico, que acompanha o tamanho que o usuário deu à peça. */
    fun travelOf(left: Boolean) =
        stickTravel * spec(if (left) PadElement.LeftStick else PadElement.RightStick).scale * layout.stick(left).travelScale

    /** Onde o centro do analógico fica em repouso (e, no modo fixo, o tempo todo). */
    fun stickRest(left: Boolean): Circle = if (left) leftStickZone else rightStickZone

    fun knobOf(left: Boolean) = stickKnobRadius * spec(if (left) PadElement.LeftStick else PadElement.RightStick).scale

    // ---- Direcional
    val dpad = circle(PadElement.Dpad, 0.20f * h)

    // ---- ABXY (Xbox: A embaixo, B à direita, X à esquerda, Y em cima)
    val faceCenter = circle(PadElement.Face, 0.17f * h)
    private val faceRadius = 0.08f * h * spec(PadElement.Face).scale
    val faceButtons: List<RoundButton> = listOf(
        RoundButton(PadButtons.A, "A", Circle(faceCenter.cx, faceCenter.cy + faceCenter.r, faceRadius)),
        RoundButton(PadButtons.B, "B", Circle(faceCenter.cx + faceCenter.r, faceCenter.cy, faceRadius)),
        RoundButton(PadButtons.X, "X", Circle(faceCenter.cx - faceCenter.r, faceCenter.cy, faceRadius)),
        RoundButton(PadButtons.Y, "Y", Circle(faceCenter.cx, faceCenter.cy - faceCenter.r, faceRadius)),
    )

    // ---- Centro
    val back = RoundButton(PadButtons.BACK, "back", circle(PadElement.Back, 0.05f * h))
    val guide = RoundButton(PadButtons.GUIDE, "guide", circle(PadElement.Guide, 0.065f * h))
    val start = RoundButton(PadButtons.START, "start", circle(PadElement.Start, 0.05f * h))
    val centerButtons = listOf(back, guide, start)

    // ---- Ombros e gatilhos (nos cantos de cima)
    private fun box(element: PadElement): Box {
        val s = spec(element)
        val halfW = 0.07f * width * s.scale
        val halfH = 0.08f * h * s.scale
        return Box(s.x * width - halfW, s.y * h - halfH, s.x * width + halfW, s.y * h + halfH)
    }

    val lt = box(PadElement.LT)
    val lb = box(PadElement.LB)
    val rb = box(PadElement.RB)
    val rt = box(PadElement.RT)

    /** Quanto o dedo precisa descer a partir do topo do gatilho para chegar a 100%. */
    val triggerReach = 0.30f * h

    /** Cada gatilho vale de 0 (dedo no topo) a 1 (dedo a [triggerReach] abaixo do topo). */
    fun triggerValue(left: Boolean, y: Float): Float {
        val top = (if (left) lt else rt).top
        return ((y - top) / triggerReach).coerceIn(0f, 1f)
    }

    /** O que está sob o dedo, na ordem de prioridade (botões pequenos antes das zonas grandes). */
    fun hit(x: Float, y: Float): Hit? {
        // Botões do centro e ABXY têm uma folga para dedos grandes.
        centerButtons.firstOrNull { visible(elementOf(it.mask)) && it.shape.contains(x, y, 1.3f) }
            ?.let { return Hit.Button(it.mask) }
        if (visible(PadElement.Face)) nearestFace(x, y)?.let { return Hit.Button(it.mask) }
        shoulderHit(x, y)?.let { return it }
        if (visible(PadElement.Dpad) && dpad.contains(x, y, 1.1f)) return Hit.Dpad(dpadMask(x, y))
        if (visible(PadElement.LeftStick) && leftStickZone.contains(x, y)) return Hit.Stick(left = true)
        if (visible(PadElement.RightStick) && rightStickZone.contains(x, y)) return Hit.Stick(left = false)
        return null
    }

    /** Só botões (ABXY, direcional, centro, ombros): usado quando o dedo desliza de um para outro. */
    fun hitButtonOnly(x: Float, y: Float): Hit? = hit(x, y)?.takeIf { it is Hit.Button || it is Hit.Dpad }

    private fun elementOf(mask: Int) = when (mask) {
        PadButtons.BACK -> PadElement.Back
        PadButtons.GUIDE -> PadElement.Guide
        else -> PadElement.Start
    }

    private fun nearestFace(x: Float, y: Float): RoundButton? =
        faceButtons.filter { it.shape.contains(x, y, 1.35f) }
            .minByOrNull { hypot(x - it.shape.cx, y - it.shape.cy) }

    private fun shoulderHit(x: Float, y: Float): Hit? {
        val slop = 0.015f * h
        return when {
            visible(PadElement.LB) && lb.contains(x, y, slop) -> Hit.Button(PadButtons.LEFT_SHOULDER)
            visible(PadElement.RB) && rb.contains(x, y, slop) -> Hit.Button(PadButtons.RIGHT_SHOULDER)
            visible(PadElement.LT) && lt.contains(x, y, slop) -> Hit.Trigger(left = true)
            visible(PadElement.RT) && rt.contains(x, y, slop) -> Hit.Trigger(left = false)
            else -> null
        }
    }

    // ---- Editor

    /** Retângulo que envolve a peça (para selecioná-la e destacá-la no editor). */
    fun bounds(element: PadElement): Box = when (element) {
        PadElement.LeftStick -> leftStickZone.box()
        PadElement.RightStick -> rightStickZone.box()
        PadElement.Dpad -> dpad.box()
        PadElement.Face -> {
            val r = faceCenter.r + faceRadius
            Box(faceCenter.cx - r, faceCenter.cy - r, faceCenter.cx + r, faceCenter.cy + r)
        }
        PadElement.Back -> back.shape.box()
        PadElement.Guide -> guide.shape.box()
        PadElement.Start -> start.shape.box()
        PadElement.LT -> lt
        PadElement.LB -> lb
        PadElement.RB -> rb
        PadElement.RT -> rt
    }

    /**
     * A peça sob o dedo no editor, **inclusive as escondidas**. Em empate (peças sobrepostas)
     * vence a de menor área: a peça pequena não fica presa debaixo de uma zona grande.
     */
    fun elementAt(x: Float, y: Float): PadElement? =
        PadElement.entries
            .filter { bounds(it).contains(x, y) }
            .minByOrNull { bounds(it).let { b -> b.width * b.height } }

    private fun Circle.box() = Box(cx - r, cy - r, cx + r, cy + r)

    /** Direção do direcional (8 vias): nada no centro morto, diagonal quando os dois eixos pesam. */
    fun dpadMask(x: Float, y: Float): Int {
        val dx = x - dpad.cx
        val dy = y - dpad.cy
        val dist = hypot(dx, dy)
        if (dist < dpad.r * 0.28f) return 0
        var mask = 0
        // Um eixo só conta se for relevante em relação ao total: a diagonal fica nos ~45°.
        if (abs(dx) > dist * 0.38f) mask = mask or if (dx > 0) PadButtons.DPAD_RIGHT else PadButtons.DPAD_LEFT
        if (abs(dy) > dist * 0.38f) mask = mask or if (dy > 0) PadButtons.DPAD_DOWN else PadButtons.DPAD_UP
        return mask
    }
}
