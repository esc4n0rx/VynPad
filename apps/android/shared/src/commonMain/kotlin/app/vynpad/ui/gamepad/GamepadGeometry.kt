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
 * É código puro (sem Compose) para ser testado sem aparelho.
 */
class GamepadGeometry(val width: Float, val height: Float) {
    private val h = height

    // ---- Analógicos
    val leftStickZone = Circle(0.14f * width, 0.47f * h, 0.25f * h)
    val rightStickZone = Circle(0.68f * width, 0.75f * h, 0.23f * h)

    /** Curso do analógico: a distância do centro até a deflexão máxima. */
    val stickTravel = 0.11f * h

    /** Raio do polegar (cabeça) desenhado no analógico. */
    val stickKnobRadius = 0.085f * h

    // ---- Direcional
    val dpad = Circle(0.32f * width, 0.74f * h, 0.20f * h)

    // ---- ABXY (Xbox: A embaixo, B à direita, X à esquerda, Y em cima)
    val faceCenter = Circle(0.86f * width, 0.46f * h, 0.17f * h)
    private val faceRadius = 0.08f * h
    val faceButtons: List<RoundButton> = listOf(
        RoundButton(PadButtons.A, "A", Circle(faceCenter.cx, faceCenter.cy + faceCenter.r, faceRadius)),
        RoundButton(PadButtons.B, "B", Circle(faceCenter.cx + faceCenter.r, faceCenter.cy, faceRadius)),
        RoundButton(PadButtons.X, "X", Circle(faceCenter.cx - faceCenter.r, faceCenter.cy, faceRadius)),
        RoundButton(PadButtons.Y, "Y", Circle(faceCenter.cx, faceCenter.cy - faceCenter.r, faceRadius)),
    )

    // ---- Centro
    val back = RoundButton(PadButtons.BACK, "back", Circle(0.43f * width, 0.30f * h, 0.05f * h))
    val guide = RoundButton(PadButtons.GUIDE, "guide", Circle(0.50f * width, 0.30f * h, 0.065f * h))
    val start = RoundButton(PadButtons.START, "start", Circle(0.57f * width, 0.30f * h, 0.05f * h))
    val centerButtons = listOf(back, guide, start)

    // ---- Ombros e gatilhos (nos cantos de cima)
    private val topEdge = 0.04f * h
    private val shoulderBottom = 0.20f * h
    val lt = Box(0.03f * width, topEdge, 0.17f * width, shoulderBottom)
    val lb = Box(0.19f * width, topEdge, 0.33f * width, shoulderBottom)
    val rb = Box(0.67f * width, topEdge, 0.81f * width, shoulderBottom)
    val rt = Box(0.83f * width, topEdge, 0.97f * width, shoulderBottom)

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
        centerButtons.firstOrNull { it.shape.contains(x, y, 1.3f) }?.let { return Hit.Button(it.mask) }
        nearestFace(x, y)?.let { return Hit.Button(it.mask) }
        shoulderHit(x, y)?.let { return it }
        if (dpad.contains(x, y, 1.1f)) return Hit.Dpad(dpadMask(x, y))
        if (leftStickZone.contains(x, y)) return Hit.Stick(left = true)
        if (rightStickZone.contains(x, y)) return Hit.Stick(left = false)
        return null
    }

    /** Só botões (ABXY, direcional, centro, ombros): usado quando o dedo desliza de um para outro. */
    fun hitButtonOnly(x: Float, y: Float): Hit? = hit(x, y)?.takeIf { it is Hit.Button || it is Hit.Dpad }

    private fun nearestFace(x: Float, y: Float): RoundButton? =
        faceButtons.filter { it.shape.contains(x, y, 1.35f) }
            .minByOrNull { hypot(x - it.shape.cx, y - it.shape.cy) }

    private fun shoulderHit(x: Float, y: Float): Hit? {
        val slop = 0.015f * h
        return when {
            lb.contains(x, y, slop) -> Hit.Button(PadButtons.LEFT_SHOULDER)
            rb.contains(x, y, slop) -> Hit.Button(PadButtons.RIGHT_SHOULDER)
            lt.contains(x, y, slop) -> Hit.Trigger(left = true)
            rt.contains(x, y, slop) -> Hit.Trigger(left = false)
            else -> null
        }
    }

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
