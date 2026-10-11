package app.vynpad.ui.gamepad

import app.vynpad.input.PadButtons
import kotlin.math.abs
import kotlin.math.hypot

/** Círculo em pixels. */
data class Circle(val cx: Float, val cy: Float, val r: Float) {
    fun contains(x: Float, y: Float, scale: Float = 1f): Boolean = hypot(x - cx, y - cy) <= r * scale

    fun box() = Box(cx - r, cy - r, cx + r, cy + r)
}

/** Retângulo em pixels. */
data class Box(val left: Float, val top: Float, val right: Float, val bottom: Float) {
    val width get() = right - left
    val height get() = bottom - top

    fun contains(x: Float, y: Float, slop: Float = 0f): Boolean = x >= left - slop && x <= right + slop && y >= top - slop && y <= bottom + slop
}

/** Botão redondo do controle: a máscara que ele aciona e o rótulo desenhado nele. */
data class RoundButton(val mask: Int, val label: String, val shape: Circle)

/** Uma peça do layout já colocada na tela de `width` × `height` pixels. */
sealed class Part(val widget: PadWidget) {
    val id: String get() = widget.id
    val visible: Boolean get() = widget.spec.visible

    /** Retângulo que envolve a peça (para selecioná-la e destacá-la no editor). */
    abstract val bounds: Box

    /** Quem vence quando duas peças se sobrepõem sob o dedo: menor número primeiro. */
    abstract val priority: Int
}

class StickPart(widget: PadWidget, val zone: Circle) : Part(widget) {
    override val bounds get() = zone.box()
    override val priority get() = 4
}

class DpadPart(widget: PadWidget, val zone: Circle) : Part(widget) {
    override val bounds get() = zone.box()
    override val priority get() = 3
}

class FacePart(widget: PadWidget, val center: Circle, val radius: Float, val buttons: List<RoundButton>) : Part(widget) {
    override val bounds get() = Box(center.cx - center.r - radius, center.cy - center.r - radius, center.cx + center.r + radius, center.cy + center.r + radius)
    override val priority get() = 1
}

/** Botão em disco (Back, Guide, Start...). */
class RoundPart(widget: PadWidget, val button: RoundButton) : Part(widget) {
    override val bounds get() = button.shape.box()
    override val priority get() = 0
}

/** Botão em pílula (ombros). */
class PillPart(widget: PadWidget, val box: Box) : Part(widget) {
    override val bounds get() = box
    override val priority get() = 2
}

/** Gatilho ou pedal; no pedal o dedo percorre a altura toda da peça ([tall]). */
class TriggerPart(widget: PadWidget, val box: Box, val tall: Boolean = false) : Part(widget) {
    override val bounds get() = box
    override val priority get() = 2
}

/** Onde um toque caiu. */
sealed interface Hit {
    /** Botão redondo, ombro ou botão do centro. */
    data class Button(val mask: Int, val latchId: String? = null) : Hit

    /** Direcional: `mask` pode ser 0 (centro morto), uma direção ou uma diagonal. */
    data class Dpad(val mask: Int) : Hit

    data class Stick(val part: StickPart) : Hit

    data class Trigger(val part: TriggerPart) : Hit
}

/**
 * O layout do controle para uma tela de `width` × `height` pixels, em paisagem: tudo em
 * frações da altura e da largura, então vale de um celular de 5" a um tablet de 11".
 * As peças ficam onde o [layout] manda (o padrão reproduz o desenho original).
 * É código puro (sem Compose) para ser testado sem aparelho.
 */
class GamepadGeometry(val width: Float, val height: Float, val layout: PadLayout = PadLayouts.default()) {
    private val h = height

    /** Todas as peças do layout, na ordem de desenho (as escondidas também: o editor as mostra). */
    val parts: List<Part> = layout.widgets.map(::place)

    private val byPriority = parts.sortedBy { it.priority }

    private fun place(w: PadWidget): Part {
        val s = w.spec
        val cx = s.x * width
        val cy = s.y * h
        return when (w.kind) {
            WidgetKind.Stick -> StickPart(w, Circle(cx, cy, (if (w.left) 0.25f else 0.23f) * h * s.scale))

            WidgetKind.Dpad -> DpadPart(w, Circle(cx, cy, 0.20f * h * s.scale))

            WidgetKind.Face -> {
                val center = Circle(cx, cy, 0.17f * h * s.scale)
                val radius = 0.08f * h * s.scale
                // Xbox: A embaixo, B à direita, X à esquerda, Y em cima.
                FacePart(
                    w,
                    center,
                    radius,
                    listOf(
                        RoundButton(PadButtons.A, "A", Circle(center.cx, center.cy + center.r, radius)),
                        RoundButton(PadButtons.B, "B", Circle(center.cx + center.r, center.cy, radius)),
                        RoundButton(PadButtons.X, "X", Circle(center.cx - center.r, center.cy, radius)),
                        RoundButton(PadButtons.Y, "Y", Circle(center.cx, center.cy - center.r, radius)),
                    ),
                )
            }

            WidgetKind.Button -> when (w.style) {
                ButtonStyle.Pill -> PillPart(w, pill(cx, cy, s.scale))
                ButtonStyle.Round -> RoundPart(w, RoundButton(w.mask, w.displayLabel(), Circle(cx, cy, 0.05f * h * s.scale)))
                ButtonStyle.RoundLarge -> RoundPart(w, RoundButton(w.mask, w.displayLabel(), Circle(cx, cy, 0.065f * h * s.scale)))
            }

            WidgetKind.Trigger -> TriggerPart(w, pill(cx, cy, s.scale))

            WidgetKind.Pedal -> {
                val halfW = 0.10f * h * s.scale
                val halfH = 0.30f * h * s.scale
                TriggerPart(w, Box(cx - halfW, cy - halfH, cx + halfW, cy + halfH), tall = true)
            }
        }
    }

    private fun pill(cx: Float, cy: Float, scale: Float): Box {
        val halfW = 0.07f * width * scale
        val halfH = 0.08f * h * scale
        return Box(cx - halfW, cy - halfH, cx + halfW, cy + halfH)
    }

    // ---- Analógicos

    /** Curso do analógico com a peça no tamanho original (a distância do centro à deflexão máxima). */
    val stickTravel = 0.11f * h

    /** Raio do polegar (cabeça) desenhado no analógico, no tamanho original. */
    val stickKnobRadius = 0.085f * h

    /** Curso deste analógico, que acompanha o tamanho que o usuário deu à peça. */
    fun travelOf(part: StickPart) = stickTravel * part.widget.spec.scale * part.widget.stick.travelScale

    fun knobOf(part: StickPart) = stickKnobRadius * part.widget.spec.scale

    // ---- Atalhos para as peças do controle padrão (o primeiro analógico/gatilho de cada lado)

    private fun stickOf(left: Boolean) = parts.firstNotNullOfOrNull { (it as? StickPart)?.takeIf { p -> p.widget.left == left } }

    private fun triggerOf(left: Boolean) = parts.firstNotNullOfOrNull { (it as? TriggerPart)?.takeIf { p -> p.widget.left == left } }

    private fun roundOf(mask: Int) = parts.firstNotNullOfOrNull { (it as? RoundPart)?.takeIf { p -> p.button.mask == mask } }

    private fun pillOf(mask: Int) = parts.firstNotNullOfOrNull { (it as? PillPart)?.takeIf { p -> p.widget.mask == mask } }

    private fun fallback(id: String): Part = place(PadLayouts.DefaultWidgets.first { it.id == id })

    val leftStickZone: Circle get() = (stickOf(true) ?: fallback(PadIds.LeftStick) as StickPart).zone
    val rightStickZone: Circle get() = (stickOf(false) ?: fallback(PadIds.RightStick) as StickPart).zone

    fun travelOf(left: Boolean): Float = travelOf(stickOf(left) ?: fallback(if (left) PadIds.LeftStick else PadIds.RightStick) as StickPart)

    val dpad: Circle get() = (parts.filterIsInstance<DpadPart>().firstOrNull() ?: fallback(PadIds.Dpad) as DpadPart).zone

    private val face get() = parts.filterIsInstance<FacePart>().firstOrNull() ?: fallback(PadIds.Face) as FacePart
    val faceCenter: Circle get() = face.center
    val faceButtons: List<RoundButton> get() = face.buttons

    val back: RoundButton get() = (roundOf(PadButtons.BACK) ?: fallback(PadIds.Back) as RoundPart).button
    val guide: RoundButton get() = (roundOf(PadButtons.GUIDE) ?: fallback(PadIds.Guide) as RoundPart).button
    val start: RoundButton get() = (roundOf(PadButtons.START) ?: fallback(PadIds.Start) as RoundPart).button

    val lt: Box get() = (triggerOf(true) ?: fallback(PadIds.LT) as TriggerPart).box
    val rt: Box get() = (triggerOf(false) ?: fallback(PadIds.RT) as TriggerPart).box
    val lb: Box get() = (pillOf(PadButtons.LEFT_SHOULDER) ?: fallback(PadIds.LB) as PillPart).box
    val rb: Box get() = (pillOf(PadButtons.RIGHT_SHOULDER) ?: fallback(PadIds.RB) as PillPart).box

    // ---- Gatilhos

    /** Quanto o dedo precisa descer a partir do topo do gatilho para chegar a 100%. */
    val triggerReach = 0.30f * h

    /** O gatilho vale de 0 (dedo no topo) a 1 (dedo a [triggerReach] abaixo do topo). */
    fun triggerValue(part: TriggerPart, y: Float): Float = ((y - part.box.top) / (if (part.tall) part.box.height else triggerReach)).coerceIn(0f, 1f)

    // ---- Toque

    /** O que está sob o dedo, na ordem de prioridade (botões pequenos antes das zonas grandes). */
    fun hit(x: Float, y: Float): Hit? {
        for (part in byPriority) {
            if (!part.visible) continue
            val hit = when (part) {
                is RoundPart -> Hit.Button(part.button.mask, part.latchId()).takeIf { part.button.shape.contains(x, y, 1.3f) }

                // Botões do centro e ABXY têm uma folga para dedos grandes.
                is FacePart -> nearestFace(part, x, y)?.let { Hit.Button(it.mask) }

                is PillPart -> Hit.Button(part.widget.mask, part.latchId()).takeIf { part.box.contains(x, y, SHOULDER_SLOP * h) }

                is TriggerPart -> Hit.Trigger(part).takeIf { part.box.contains(x, y, SHOULDER_SLOP * h) }

                is DpadPart -> Hit.Dpad(dpadMask(part.zone, x, y)).takeIf { part.zone.contains(x, y, 1.1f) }

                is StickPart -> Hit.Stick(part).takeIf { part.zone.contains(x, y) }
            }
            if (hit != null) return hit
        }
        return null
    }

    /** Só botões (ABXY, direcional, centro, ombros): usado quando o dedo desliza de um para outro. */
    fun hitButtonOnly(x: Float, y: Float): Hit? = hit(x, y)?.takeIf { it is Hit.Button || it is Hit.Dpad }

    private fun Part.latchId(): String? = widget.id.takeIf { widget.latch }

    private fun nearestFace(part: FacePart, x: Float, y: Float): RoundButton? = part.buttons.filter { it.shape.contains(x, y, 1.35f) }
        .minByOrNull { hypot(x - it.shape.cx, y - it.shape.cy) }

    // ---- Editor

    /** Retângulo que envolve a peça [id]; `null` se o layout não a tem. */
    fun bounds(id: String): Box? = parts.firstOrNull { it.id == id }?.bounds

    /**
     * A peça sob o dedo no editor, **inclusive as escondidas**. Em empate (peças sobrepostas)
     * vence a de menor área: a peça pequena não fica presa debaixo de uma zona grande.
     */
    fun elementAt(x: Float, y: Float): String? = parts
        .filter { it.bounds.contains(x, y) }
        .minByOrNull { it.bounds.let { b -> b.width * b.height } }
        ?.id

    /** Direção do direcional padrão (8 vias): nada no centro morto, diagonal quando os dois eixos pesam. */
    fun dpadMask(x: Float, y: Float): Int = dpadMask(dpad, x, y)

    private fun dpadMask(zone: Circle, x: Float, y: Float): Int {
        val dx = x - zone.cx
        val dy = y - zone.cy
        val dist = hypot(dx, dy)
        if (dist < zone.r * 0.28f) return 0
        var mask = 0
        // Um eixo só conta se for relevante em relação ao total: a diagonal fica nos ~45°.
        if (abs(dx) > dist * 0.38f) mask = mask or if (dx > 0) PadButtons.DPAD_RIGHT else PadButtons.DPAD_LEFT
        if (abs(dy) > dist * 0.38f) mask = mask or if (dy > 0) PadButtons.DPAD_DOWN else PadButtons.DPAD_UP
        return mask
    }

    private companion object {
        /** Folga do toque em ombros e gatilhos, em fração da altura. */
        const val SHOULDER_SLOP = 0.015f
    }
}
