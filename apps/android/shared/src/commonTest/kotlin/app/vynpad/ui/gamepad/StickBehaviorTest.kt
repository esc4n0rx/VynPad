package app.vynpad.ui.gamepad

import app.vynpad.input.GamepadState
import app.vynpad.input.PadButtons
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class StickBehaviorTest {
    private val w = 2400f
    private val h = 1080f

    private fun rig(layout: PadLayout): Triple<GamepadGeometry, TouchRouter, GamepadState> {
        val g = GamepadGeometry(w, h, layout)
        val state = GamepadState()
        return Triple(g, TouchRouter({ g }, state, PadVisuals()), state)
    }

    @Test
    fun zonaMortaRadialReescalaSemSalto() {
        assertEquals(0f to 0f, StickMath.deadzone(0.1f, 0f, 0.2f))
        val (x, _) = StickMath.deadzone(1f, 0f, 0.2f)
        assertEquals(1f, x, 1e-5f)
        val (mid, _) = StickMath.deadzone(0.6f, 0f, 0.2f)
        assertEquals(0.5f, mid, 1e-5f)
        assertEquals(0.3f to 0.4f, StickMath.deadzone(0.3f, 0.4f, 0f))
    }

    @Test
    fun analogicoFlutuanteCentraNoDedo() {
        val (g, router, state) = rig(PadLayouts.default())
        val z = g.leftStickZone
        router.down(1, z.cx + 50f, z.cy, 0)
        assertEquals(0, state.lx)
        router.move(1, z.cx + 50f + g.travelOf(true), z.cy)
        assertEquals(32767, state.lx)
    }

    @Test
    fun analogicoFixoUsaOCentroDaZona() {
        val fixed = PadLayouts.default().withStick(true, StickConfig(floating = false))
        val (g, router, state) = rig(fixed)
        val z = g.leftStickZone
        // O primeiro toque, meio curso à direita do centro, já deflete o analógico.
        router.down(1, z.cx + g.travelOf(true) / 2, z.cy, 0)
        assertTrue(state.lx in 15000..17500, "lx=${state.lx}")
        router.up(1, 10)
        assertEquals(0, state.lx)
    }

    @Test
    fun cursoMenorSaturaMaisCedo() {
        val short = PadLayouts.default().withStick(true, StickConfig(travelScale = 0.6f))
        val (g, router, state) = rig(short)
        val z = g.leftStickZone
        router.down(1, z.cx, z.cy, 0)
        router.move(1, z.cx + g.stickTravel * 0.6f, z.cy)
        assertEquals(32767, state.lx)
    }

    @Test
    fun zonaMortaDeToqueSilenciaPequenosMovimentos() {
        val dz = PadLayouts.default().withStick(true, StickConfig(deadzone = 0.3f))
        val (g, router, state) = rig(dz)
        val z = g.leftStickZone
        router.down(1, z.cx, z.cy, 0)
        router.move(1, z.cx + g.travelOf(true) * 0.2f, z.cy)
        assertEquals(0, state.lx)
        router.move(1, z.cx + g.travelOf(true), z.cy)
        assertEquals(32767, state.lx)
    }

    @Test
    fun cliqueL3FuncionaNoModoFixoMesmoForaDoCentro() {
        val fixed = PadLayouts.default().withStick(true, StickConfig(floating = false))
        val (g, router, state) = rig(fixed)
        val z = g.leftStickZone
        val x = z.cx + g.travelOf(true) * 0.8f
        router.down(1, x, z.cy, 0)
        router.up(1, 80)
        router.down(2, x, z.cy, 150)
        assertEquals(PadButtons.LEFT_THUMB, state.buttons and PadButtons.LEFT_THUMB)
    }

    @Test
    fun ajusteDoEditorFicaNosLimites() {
        val l = PadEditing.setStick(PadLayouts.default(), false, StickConfig(travelScale = 9f, deadzone = 9f))
        assertEquals(PadLayouts.MAX_TRAVEL, l.stick(false).travelScale)
        assertEquals(PadLayouts.MAX_DEADZONE, l.stick(false).deadzone)
        assertEquals(StickConfig(), l.stick(true))
    }
}
