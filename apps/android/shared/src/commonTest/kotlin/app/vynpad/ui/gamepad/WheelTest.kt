package app.vynpad.ui.gamepad

import app.vynpad.input.GamepadState
import app.vynpad.input.MotionMath
import app.vynpad.input.Vec3
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

/** O volante: gira com o dedo e/ou com a inclinação do celular e vira o eixo X de um analógico. */
class WheelTest {
    private val w = 2400f
    private val h = 1080f

    private fun layoutWith(wheel: PadWidget = PadLayouts.newWheel()): Pair<PadLayout, String> {
        // Corrida: sem os analógicos, que ocupariam o mesmo lugar.
        val base = listOf(PadIds.LeftStick, PadIds.RightStick).fold(PadLayouts.default()) { l, id -> PadEditing.setVisible(l, id, false) }
        return assertNotNull(PadEditing.add(base, wheel))
    }

    private class Rig(val g: GamepadGeometry, val router: TouchRouter, val state: GamepadState, val visuals: PadVisuals)

    private fun rig(layout: PadLayout): Rig {
        val g = GamepadGeometry(w, h, layout)
        val state = GamepadState()
        val visuals = PadVisuals()
        return Rig(g, TouchRouter({ g }, state, visuals), state, visuals)
    }

    private fun Rig.zone(id: String) = (g.parts.first { it.id == id } as WheelPart).zone

    private fun at(zone: Circle, degrees: Float, radius: Float = 0.8f): Pair<Float, Float> {
        val a = degrees * PI.toFloat() / 180f
        return zone.cx + zone.r * radius * cos(a) to zone.cy + zone.r * radius * sin(a)
    }

    private fun assertNear(expected: Int, actual: Int, tolerance: Int) {
        assertTrue(kotlin.math.abs(expected - actual) <= tolerance, "esperava $expected ±$tolerance, veio $actual")
    }

    @Test
    fun rolagemDoAcelerometro() {
        assertEquals(0f, MotionMath.roll(Vec3(0f, 9.8f, 0f)), 1e-5f)
        // Girou o aparelho 30° no sentido horário: a gravidade passa a "puxar" para o lado esquerdo do aparelho.
        val a = 30f * PI.toFloat() / 180f
        assertEquals(a, MotionMath.roll(Vec3(-9.8f * sin(a), 9.8f * cos(a), 0f)), 1e-4f)
        assertEquals(-a, MotionMath.roll(Vec3(9.8f * sin(a), 9.8f * cos(a), 0f)), 1e-4f)
    }

    @Test
    fun dedoGiraOVolanteEViraOEixoDoAnalogicoEsquerdo() {
        val (layout, id) = layoutWith(PadLayouts.newWheel().copy(wheel = WheelConfig(range = 90f, tilt = false, deadzone = 0f)))
        val r = rig(layout)
        val zone = r.zone(id)
        // Pega o volante no topo e arrasta 45° no sentido horário: metade do curso (90° = 100%).
        val (x0, y0) = at(zone, 0f)
        r.router.down(1, x0, y0, 0)
        val (x1, y1) = at(zone, 45f)
        r.router.move(1, x1, y1)
        assertNear(16384, r.state.lx, 80)
        assertEquals(0.5f, r.visuals.wheel, 0.01f)
        val (x2, y2) = at(zone, 90f)
        r.router.move(1, x2, y2)
        assertEquals(32767, r.state.lx, "trava no limite")
        r.router.up(1, 100)
        assertEquals(0, r.state.lx, "sem dedo nem inclinação o volante volta ao centro")
        assertTrue(r.visuals.wheelSpring, "o desenho volta com mola")
    }

    @Test
    fun arrastarNoSentidoAntiHorarioEsterceParaEsquerda() {
        val (layout, id) = layoutWith(PadLayouts.newWheel().copy(wheel = WheelConfig(tilt = false, deadzone = 0f)))
        val r = rig(layout)
        val zone = r.zone(id)
        val (x0, y0) = at(zone, 0f)
        r.router.down(1, x0, y0, 0)
        val (x1, y1) = at(zone, -45f)
        r.router.move(1, x1, y1)
        assertTrue(r.state.lx < 0)
    }

    @Test
    fun voltaCompletaNaoDaSaltoNaVirada180() {
        val (layout, id) = layoutWith(PadLayouts.newWheel().copy(wheel = WheelConfig(range = 180f, tilt = false, deadzone = 0f)))
        val r = rig(layout)
        val zone = r.zone(id)
        val (x0, y0) = at(zone, 0f)
        r.router.down(1, x0, y0, 0)
        // Passos de 40° até além de 180°: o ângulo acumula, não pula de sinal.
        var previous = 0
        for (deg in listOf(40f, 80f, 120f, 160f)) {
            val (x, y) = at(zone, deg)
            r.router.move(1, x, y)
            assertTrue(r.state.lx >= previous, "monotônico em $deg")
            previous = r.state.lx
        }
        assertNear(32767 * 160 / 180, r.state.lx, 120)
    }

    @Test
    fun inclinacaoGiraOVolanteSemDedo() {
        val (layout, _) = layoutWith(PadLayouts.newWheel().copy(wheel = WheelConfig(range = 90f, touch = true, tilt = true, deadzone = 0f)))
        val r = rig(layout)
        r.router.setTilt(45f * PI.toFloat() / 180f)
        assertNear(16384, r.state.lx, 80)
        r.router.setTilt(-90f * PI.toFloat() / 180f)
        assertEquals(-32767, r.state.lx)
        r.router.setTilt(null)
        assertEquals(0, r.state.lx)
    }

    @Test
    fun zonaMortaDaInclinacao() {
        val (layout, _) = layoutWith(PadLayouts.newWheel().copy(wheel = WheelConfig(range = 90f, deadzone = 0.1f)))
        val r = rig(layout)
        // 4° de 90° = 4,4%: dentro dos 10% de zona morta.
        r.router.setTilt(4f * PI.toFloat() / 180f)
        assertEquals(0, r.state.lx)
        // 90° chega ao limite sem salto.
        r.router.setTilt(90f * PI.toFloat() / 180f)
        assertEquals(32767, r.state.lx)
    }

    @Test
    fun inclinacaoDesligadaOuVolanteEscondidoNaoMexe() {
        val (off, _) = layoutWith(PadLayouts.newWheel().copy(wheel = WheelConfig(tilt = false)))
        val a = rig(off)
        a.router.setTilt(1f)
        assertEquals(0, a.state.lx)

        val (layout, id) = layoutWith()
        val hidden = PadEditing.setVisible(layout, id, false)
        val b = rig(hidden)
        b.router.setTilt(1f)
        assertEquals(0, b.state.lx)
    }

    @Test
    fun dedoNoVolanteIgnoraAInclinacaoEDepoisVoltaPraElaSegundo() {
        val (layout, id) = layoutWith(PadLayouts.newWheel().copy(wheel = WheelConfig(range = 90f, deadzone = 0f)))
        val r = rig(layout)
        r.router.setTilt(45f * PI.toFloat() / 180f)
        val zone = r.zone(id)
        // O dedo pega o volante onde a inclinação o deixou e continua dali.
        val (x0, y0) = at(zone, 0f)
        r.router.down(1, x0, y0, 0)
        assertNear(16384, r.state.lx, 80)
        r.router.setTilt(-80f * PI.toFloat() / 180f)
        assertNear(16384, r.state.lx, 80)
        r.router.up(1, 50)
        assertTrue(r.state.lx < -20000, "solto o dedo, a inclinação volta a mandar")
    }

    @Test
    fun volanteDoLadoDireitoVaiParaORSX() {
        val (layout, _) = layoutWith(PadLayouts.newWheel().copy(left = false, wheel = WheelConfig(deadzone = 0f)))
        val r = rig(layout)
        r.router.setTilt(90f * PI.toFloat() / 180f)
        assertEquals(32767, r.state.rx)
        assertEquals(0, r.state.lx)
    }

    @Test
    fun volanteSemTouchNaoPegaODedo() {
        val (layout, id) = layoutWith(PadLayouts.newWheel().copy(wheel = WheelConfig(touch = false)))
        val r = rig(layout)
        val zone = r.zone(id)
        assertNull(r.g.hit(zone.cx, zone.cy + zone.r * 0.5f))
    }

    @Test
    fun botaoSobreOVolanteTemPreferencia() {
        val (layout, _) = layoutWith()
        val withButton = assertNotNull(
            PadEditing.add(layout, PadLayouts.newButton().copy(spec = ElementSpec(0.5f, 0.55f), mask = 0x1000)),
        ).first
        val g = GamepadGeometry(w, h, withButton)
        assertTrue(g.hit(0.5f * w, 0.55f * h) is Hit.Button)
    }

    @Test
    fun soHaUmVolante() {
        val (layout, id) = layoutWith()
        assertNull(PadEditing.duplicate(layout, id))
        val twice = layout.copy(widgets = layout.widgets + layout.widget(id)!!.copy(id = "outro")).sanitized()
        assertEquals(1, twice.widgets.count { it.kind == WidgetKind.Wheel })
    }

    @Test
    fun ajustesDoVolanteFicamNosLimitesEIdaEVoltaPeloArquivo() {
        val (layout, id) = layoutWith()
        val wild = PadEditing.setWheel(layout, id, WheelConfig(range = 9999f, deadzone = 9f))
        assertEquals(PadLayouts.MAX_WHEEL_RANGE, wild.widget(id)?.wheel?.range)
        assertEquals(PadLayouts.MAX_WHEEL_DEADZONE, wild.widget(id)?.wheel?.deadzone)
        val saved = PadEditing.setWheel(layout, id, WheelConfig(range = 120f, touch = false, tilt = true, deadzone = 0.1f))
        val back = assertNotNull(LayoutPackage.decode(LayoutPackage.encode(saved, null))).layout
        assertEquals(saved.widgets, back.widgets)
        val hostile = saved.copy(
            widgets = saved.widgets.map { if (it.id == id) it.copy(wheel = WheelConfig(range = Float.NaN, deadzone = -3f)) else it },
        ).sanitized()
        assertEquals(90f, hostile.widget(id)?.wheel?.range)
        assertEquals(0f, hostile.widget(id)?.wheel?.deadzone)
    }
}
