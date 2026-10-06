package app.vynpad.ui.gamepad

import app.vynpad.input.GamepadState
import app.vynpad.input.PadButtons
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class TouchRouterTest {
    private val geometry = GamepadGeometry(2400f, 1080f)
    private val state = GamepadState()
    private val visuals = PadVisuals()
    private val log = mutableListOf<String>()
    private val router = TouchRouter(
        geometry = { geometry },
        state = state,
        visuals = visuals,
        feedback = object : PadFeedback {
            override fun press() {
                log += "press"
            }

            override fun slide() {
                log += "slide"
            }

            override fun detent() {
                log += "detent"
            }
        },
    )

    private fun center(button: RoundButton) = button.shape.cx to button.shape.cy

    private fun face(mask: Int) = geometry.faceButtons.first { it.mask == mask }

    @Test
    fun fiveSimultaneousTouchesAreAllRegistered() {
        // Analógico esquerdo + A + gatilho direito + direcional para cima + ombro direito.
        val stick = geometry.leftStickZone
        router.down(1, stick.cx, stick.cy, 0)
        router.move(1, stick.cx + geometry.stickTravel, stick.cy) // todo para a direita
        val (ax, ay) = center(face(PadButtons.A))
        router.down(2, ax, ay, 0)
        // O gatilho se toca dentro da caixa e se aperta arrastando o dedo para baixo.
        val triggerX = (geometry.rt.left + geometry.rt.right) / 2
        router.down(3, triggerX, geometry.rt.top + 10f, 0)
        router.move(3, triggerX, geometry.rt.top + geometry.triggerReach)
        router.down(4, geometry.dpad.cx, geometry.dpad.cy - geometry.dpad.r * 0.7f, 0)
        router.down(5, (geometry.rb.left + geometry.rb.right) / 2, (geometry.rb.top + geometry.rb.bottom) / 2, 0)

        val s = state.snapshot()
        assertEquals(PadButtons.A or PadButtons.DPAD_UP or PadButtons.RIGHT_SHOULDER, s.buttons)
        assertEquals(32767, s.lx)
        assertEquals(0, s.ly)
        assertEquals(255, s.rt)
        assertEquals(0, s.lt)
    }

    @Test
    fun releasingEverythingReturnsToNeutral() {
        val (ax, ay) = center(face(PadButtons.A))
        router.down(1, ax, ay, 0)
        router.down(2, geometry.leftStickZone.cx, geometry.leftStickZone.cy, 0)
        router.move(2, geometry.leftStickZone.cx + 40f, geometry.leftStickZone.cy)
        router.up(1, 10)
        router.up(2, 10)
        assertEquals(0, state.snapshot().buttons)
        assertEquals(0, state.snapshot().lx)
        assertEquals(StickVisual.Rest, visuals.leftStick)
    }

    @Test
    fun aFingerSlidesFromOneButtonToTheNextWithoutLifting() {
        val (ax, ay) = center(face(PadButtons.A))
        val (bx, by) = center(face(PadButtons.B))
        router.down(1, ax, ay, 0)
        assertEquals(PadButtons.A, state.buttons)
        router.move(1, bx, by)
        assertEquals(PadButtons.B, state.buttons)
        assertTrue("slide" in log, "o deslize deve dar um toque")
        // Saiu para o vazio: solta, e voltando ao botão aperta de novo.
        router.move(1, geometry.width / 2, geometry.height * 0.9f)
        assertEquals(0, state.buttons)
        router.move(1, bx, by)
        assertEquals(PadButtons.B, state.buttons)
    }

    @Test
    fun theStickCentersWhereTheFingerLandsAndDoesNotJump() {
        val zone = geometry.leftStickZone
        // Pousa longe do centro da zona: o valor começa em zero.
        router.down(1, zone.cx + 60f, zone.cy + 40f, 0)
        assertEquals(0, state.lx)
        assertEquals(0, state.ly)
        // Mover para cima (y da tela diminui) deve dar Y positivo, e para a esquerda, X negativo.
        router.move(1, zone.cx + 60f - geometry.stickTravel, zone.cy + 40f - geometry.stickTravel)
        assertTrue(state.lx < -20000, "x=${state.lx}")
        assertTrue(state.ly > 20000, "y=${state.ly}")
    }

    @Test
    fun theStickIsClampedToTheMaximumDeflection() {
        val zone = geometry.rightStickZone
        router.down(1, zone.cx, zone.cy, 0)
        router.move(1, zone.cx + geometry.stickTravel * 5, zone.cy)
        assertEquals(32767, state.rx)
        assertTrue("detent" in log)
        // A diagonal também respeita o limite radial: nenhum eixo passa do máximo.
        router.move(1, zone.cx + geometry.stickTravel * 4, zone.cy + geometry.stickTravel * 4)
        assertTrue(state.rx <= 32767 && state.ry >= -32767)
        val radial = kotlin.math.hypot(state.rx.toFloat(), state.ry.toFloat())
        assertTrue(radial <= 32767f * 1.001f, "radial=$radial")
    }

    @Test
    fun theTriggerIsAnalogByHeight() {
        val x = (geometry.lt.left + geometry.lt.right) / 2
        router.down(1, x, geometry.lt.top, 0)
        assertEquals(0, state.lt)
        router.move(1, x, geometry.lt.top + geometry.triggerReach / 2)
        assertTrue(state.lt in 120..135, "lt=${state.lt}")
        router.move(1, x, geometry.lt.top + geometry.triggerReach * 3)
        assertEquals(255, state.lt)
        router.up(1, 100)
        assertEquals(0, state.lt)
    }

    @Test
    fun theDpadReadsEightDirections() {
        val d = geometry.dpad
        val r = d.r * 0.7f
        assertEquals(PadButtons.DPAD_UP, geometry.dpadMask(d.cx, d.cy - r))
        assertEquals(PadButtons.DPAD_DOWN, geometry.dpadMask(d.cx, d.cy + r))
        assertEquals(PadButtons.DPAD_LEFT, geometry.dpadMask(d.cx - r, d.cy))
        assertEquals(PadButtons.DPAD_RIGHT, geometry.dpadMask(d.cx + r, d.cy))
        assertEquals(PadButtons.DPAD_UP or PadButtons.DPAD_RIGHT, geometry.dpadMask(d.cx + r, d.cy - r))
        assertEquals(PadButtons.DPAD_DOWN or PadButtons.DPAD_LEFT, geometry.dpadMask(d.cx - r, d.cy + r))
        assertEquals(0, geometry.dpadMask(d.cx + 5f, d.cy - 5f), "centro morto")
    }

    @Test
    fun aDoubleTapOnTheStickHoldsL3() {
        val zone = geometry.leftStickZone
        // Primeiro toque curto, sem mover.
        router.down(1, zone.cx, zone.cy, 1_000)
        router.up(1, 1_080)
        assertEquals(0, state.buttons and PadButtons.LEFT_THUMB)
        // Segundo toque logo depois: segura L3 enquanto o dedo estiver na tela.
        router.down(2, zone.cx, zone.cy, 1_200)
        assertEquals(PadButtons.LEFT_THUMB, state.buttons and PadButtons.LEFT_THUMB)
        router.up(2, 1_900)
        assertEquals(0, state.buttons and PadButtons.LEFT_THUMB)
        // Um terceiro toque isolado, bem depois, não é duplo.
        router.down(3, zone.cx, zone.cy, 5_000)
        assertEquals(0, state.buttons and PadButtons.LEFT_THUMB)
    }

    @Test
    fun aStickHasOnlyOneFinger() {
        val zone = geometry.leftStickZone
        router.down(1, zone.cx, zone.cy, 0)
        router.move(1, zone.cx + geometry.stickTravel, zone.cy)
        router.down(2, zone.cx - 30f, zone.cy, 0)
        router.move(2, zone.cx - geometry.stickTravel, zone.cy)
        // O segundo dedo na mesma zona é ignorado: o analógico segue o primeiro.
        assertEquals(32767, state.lx)
    }

    @Test
    fun cancelAllReleasesEverything() {
        val (ax, ay) = center(face(PadButtons.A))
        router.down(1, ax, ay, 0)
        router.down(2, geometry.rightStickZone.cx, geometry.rightStickZone.cy, 0)
        router.move(2, geometry.rightStickZone.cx + 50f, geometry.rightStickZone.cy)
        router.cancelAll()
        val s = state.snapshot()
        assertEquals(0, s.buttons)
        assertEquals(0, s.rx)
        assertNotEquals(PadButtons.A, visuals.buttons)
    }

    @Test
    fun touchesOutsideAnyControlAreIgnored() {
        router.down(1, geometry.width * 0.5f, geometry.height * 0.9f, 0)
        assertEquals(0, state.buttons)
        assertTrue(log.isEmpty())
    }
}
