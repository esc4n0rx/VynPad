package app.vynpad.ui.gamepad

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import app.vynpad.input.GamepadState
import app.vynpad.input.PadButtons
import kotlin.math.hypot
import kotlin.math.roundToInt

/** Os toques que o usuário sente: o roteador pede, a plataforma vibra. */
interface PadFeedback {
    /** Dedo apertou um botão. */
    fun press()

    /** Dedo deslizou de um botão para outro. */
    fun slide()

    /** Marca de fim de curso (analógico no máximo, gatilho em 50% e 100%). */
    fun detent()

    companion object {
        val None = object : PadFeedback {
            override fun press() = Unit
            override fun slide() = Unit
            override fun detent() = Unit
        }
    }
}

/** Posição de um analógico na tela: `origin` é onde o dedo pousou (nulo = em repouso). */
data class StickVisual(val origin: Offset?, val knob: Offset) {
    companion object {
        val Rest = StickVisual(null, Offset.Zero)
    }
}

/**
 * O que a tela desenha. São estados do Compose lidos só na fase de desenho: mudar um deles
 * redesenha o controle sem recompor nada.
 */
class PadVisuals {
    var buttons by mutableIntStateOf(0)
    var lt by mutableFloatStateOf(0f)
    var rt by mutableFloatStateOf(0f)
    var leftStick by mutableStateOf(StickVisual.Rest)
    var rightStick by mutableStateOf(StickVisual.Rest)
}

/**
 * Traduz toques (por `pointerId`) em estado do controle. Cada dedo vira um "segurar":
 * - **botão/direcional**: pode deslizar para outro botão sem levantar, como num controle físico;
 * - **analógico**: o centro é onde o dedo pousou (não pula), e dois toques seguidos
 *   (o segundo segurado) acionam L3/R3;
 * - **gatilho**: a altura do dedo é a pressão (analógica).
 *
 * Não depende de Android: recebe `down/move/up` com coordenadas e tempo e escreve em
 * [GamepadState] (lido pelo envio) e [PadVisuals] (lido pelo desenho).
 */
class TouchRouter(
    private val geometry: () -> GamepadGeometry,
    private val state: GamepadState,
    private val visuals: PadVisuals,
    private val feedback: PadFeedback = PadFeedback.None,
) {
    private sealed interface Hold

    private class ButtonHold(var mask: Int) : Hold

    private class StickHold(
        val part: StickPart,
        /** Centro do analógico: onde o dedo pousou (flutuante) ou o meio da zona (fixo). */
        val origin: Offset,
        /** Onde o dedo pousou: o toque curto (L3) é medido a partir daqui, nos dois modos. */
        val downPoint: Offset,
        val deadzone: Float,
        val downAt: Long,
        val clickHeld: Boolean,
    ) : Hold {
        val left get() = part.widget.left

        /** Deflexão enviada ao PC (já com a zona morta de toque). */
        var vx = 0f
        var vy = 0f

        /** Deflexão sob o dedo, sem zona morta: é o que o polegar desenhado mostra. */
        var kx = 0f
        var ky = 0f
        var maxDeflection = 0f
        var atLimit = false
    }

    private class TriggerHold(val part: TriggerPart) : Hold {
        val left get() = part.widget.left
        var value = 0f
    }

    private val holds = HashMap<Long, Hold>()

    /** Botões que travam (peça com `latch`): id da peça → máscara, enquanto estão apertados. */
    private val latched = LinkedHashMap<String, Int>()
    private val lastTapUp = longArrayOf(NEVER, NEVER)

    fun down(id: Long, x: Float, y: Float, nowMs: Long) {
        if (holds.containsKey(id)) return
        val g = geometry()
        val hold: Hold = when (val hit = g.hit(x, y) ?: return) {
            is Hit.Button -> {
                val latchId = hit.latchId
                if (latchId != null) {
                    // Botão que trava: o toque alterna e não há dedo a acompanhar.
                    if (latched.remove(latchId) == null) latched[latchId] = hit.mask
                    feedback.press()
                    publish()
                    return
                }
                ButtonHold(hit.mask).also { feedback.press() }
            }

            is Hit.Dpad -> ButtonHold(hit.mask).also { if (hit.mask != 0) feedback.press() }

            is Hit.Stick -> {
                // Um analógico só tem um dedo: o segundo toque na mesma zona é ignorado.
                if (holds.values.any { it is StickHold && it.part.id == hit.part.id }) return
                val index = if (hit.part.widget.left) 0 else 1
                val config = hit.part.widget.stick
                val rest = hit.part.zone
                StickHold(
                    part = hit.part,
                    origin = if (config.floating) Offset(x, y) else Offset(rest.cx, rest.cy),
                    downPoint = Offset(x, y),
                    deadzone = config.deadzone,
                    downAt = nowMs,
                    clickHeld = nowMs - lastTapUp[index] <= DOUBLE_TAP_MS,
                ).also { if (it.clickHeld) feedback.press() }
            }

            is Hit.Trigger -> {
                if (holds.values.any { it is TriggerHold && it.part.id == hit.part.id }) return
                TriggerHold(hit.part).also {
                    it.value = g.triggerValue(hit.part, y)
                    feedback.press()
                }
            }
        }
        holds[id] = hold
        // Num analógico fixo o primeiro toque já pode estar fora do centro.
        if (hold is StickHold) move(id, x, y) else publish()
    }

    fun move(id: Long, x: Float, y: Float) {
        val g = geometry()
        when (val hold = holds[id] ?: return) {
            is ButtonHold -> {
                val mask = when (val hit = g.hitButtonOnly(x, y)) {
                    // Deslizar para um botão que trava não o aciona: só o toque o alterna.
                    is Hit.Button -> if (hit.latchId == null) hit.mask else 0

                    is Hit.Dpad -> hit.mask

                    else -> 0
                }
                if (mask != hold.mask) {
                    // Entrar em outro botão dá um toque; sair para o vazio só solta.
                    if (mask != 0) feedback.slide()
                    hold.mask = mask
                }
            }

            is StickHold -> {
                val dx = x - hold.origin.x
                val dy = y - hold.origin.y
                val length = hypot(dx, dy)
                val travel = g.travelOf(hold.part)
                val scale = if (length > travel) travel / length else 1f
                hold.kx = dx * scale / travel
                hold.ky = dy * scale / travel
                val (ux, uy) = StickMath.deadzone(hold.kx, hold.ky, hold.deadzone)
                hold.vx = ux
                hold.vy = uy
                hold.maxDeflection = maxOf(hold.maxDeflection, hypot(x - hold.downPoint.x, y - hold.downPoint.y) / travel)
                val limit = length >= travel
                if (limit && !hold.atLimit) feedback.detent()
                hold.atLimit = limit
            }

            is TriggerHold -> {
                val value = g.triggerValue(hold.part, y)
                if (crossed(hold.value, value, 0.5f) || crossed(hold.value, value, 0.98f)) feedback.detent()
                hold.value = value
            }
        }
        publish()
    }

    fun up(id: Long, nowMs: Long) {
        val hold = holds.remove(id) ?: return
        if (hold is StickHold) {
            val tap = nowMs - hold.downAt <= TAP_MS && hold.maxDeflection < TAP_DEFLECTION
            lastTapUp[if (hold.left) 0 else 1] = if (tap && !hold.clickHeld) nowMs else NEVER
        }
        publish()
    }

    /** Solta tudo (a janela perdeu o foco, o menu abriu, o app foi para segundo plano). */
    fun cancelAll() {
        holds.clear()
        latched.clear()
        lastTapUp.fill(NEVER)
        publish()
    }

    private fun crossed(from: Float, to: Float, at: Float) = from < at && to >= at

    /** Recalcula o estado do controle inteiro a partir de todos os dedos que estão na tela. */
    private fun publish() {
        var buttons = latched.values.fold(0) { all, mask -> all or mask }
        var lt = 0f
        var rt = 0f
        var left = StickVisual.Rest
        var right = StickVisual.Rest
        var lx = 0
        var ly = 0
        var rx = 0
        var ry = 0
        val g = geometry()
        for (hold in holds.values) {
            when (hold) {
                is ButtonHold -> buttons = buttons or hold.mask

                is TriggerHold -> if (hold.left) lt = maxOf(lt, hold.value) else rt = maxOf(rt, hold.value)

                is StickHold -> {
                    val axisX = (hold.vx.coerceIn(-1f, 1f) * AXIS_MAX).roundToInt()
                    // O eixo Y do XInput é positivo para cima; o da tela é positivo para baixo.
                    val axisY = (-hold.vy.coerceIn(-1f, 1f) * AXIS_MAX).roundToInt()
                    val travel = g.travelOf(hold.part)
                    val visual = StickVisual(hold.origin, Offset(hold.kx * travel, hold.ky * travel))
                    if (hold.left) {
                        lx = axisX
                        ly = axisY
                        left = visual
                        if (hold.clickHeld) buttons = buttons or PadButtons.LEFT_THUMB
                    } else {
                        rx = axisX
                        ry = axisY
                        right = visual
                        if (hold.clickHeld) buttons = buttons or PadButtons.RIGHT_THUMB
                    }
                }
            }
        }
        state.buttons = buttons
        state.lt = (lt * TRIGGER_MAX).roundToInt()
        state.rt = (rt * TRIGGER_MAX).roundToInt()
        state.lx = lx
        state.ly = ly
        state.rx = rx
        state.ry = ry
        state.commit()
        visuals.buttons = buttons
        visuals.lt = lt
        visuals.rt = rt
        visuals.leftStick = left
        visuals.rightStick = right
    }

    private companion object {
        const val NEVER = Long.MIN_VALUE / 2
        const val DOUBLE_TAP_MS = 300L
        const val TAP_MS = 250L
        const val TAP_DEFLECTION = 0.35f
        const val AXIS_MAX = 32767
        const val TRIGGER_MAX = 255
    }
}
