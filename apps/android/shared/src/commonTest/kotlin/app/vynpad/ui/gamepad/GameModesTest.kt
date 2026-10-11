package app.vynpad.ui.gamepad

import app.vynpad.input.GamepadState
import app.vynpad.input.PadButtons
import kotlin.math.PI
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

/** Os modos prontos (Corrida, FPS): montam o que prometem e dá para jogá-los de verdade. */
class GameModesTest {
    private val w = 2400f
    private val h = 1080f
    private val texts = PresetTexts(brake = "FREIO", gas = "ACEL", shiftDown = "-", shiftUp = "+", sprint = "CORRER", melee = "FACA")

    private fun racing() = PadLayouts.fromPreset(PadPreset.Racing, "r", "Corrida", texts)

    private fun fps() = PadLayouts.fromPreset(PadPreset.Fps, "f", "FPS", texts)

    private fun rig(layout: PadLayout): Triple<GamepadGeometry, TouchRouter, GamepadState> {
        val g = GamepadGeometry(w, h, layout)
        val state = GamepadState()
        return Triple(g, TouchRouter({ g }, state, PadVisuals()), state)
    }

    @Test
    fun corridaTemVolantePedaisEMarchas() {
        val l = racing()
        assertEquals(1, l.widgets.count { it.kind == WidgetKind.Wheel })
        val pedals = l.widgets.filter { it.kind == WidgetKind.Pedal }
        assertEquals(setOf(true, false), pedals.map { it.left }.toSet())
        assertEquals("FREIO", pedals.first { it.left }.label)
        assertEquals("ACEL", pedals.first { !it.left }.label)
        assertEquals("-", l.widget(PadIds.LB)?.displayLabel())
        assertEquals("+", l.widget(PadIds.RB)?.displayLabel())
        // Sem analógicos nem gatilhos de topo: o volante e os pedais os substituem.
        for (id in listOf(PadIds.LeftStick, PadIds.RightStick, PadIds.LT, PadIds.RT, PadIds.Guide)) {
            assertEquals(false, l.spec(id).visible, id)
        }
        assertTrue(l.widget("wheel")?.wheel?.tilt == true && l.widget("wheel")?.wheel?.touch == true)
    }

    @Test
    fun corridaEJogavelPedalEMarchaAoMesmoTempo() {
        val (g, router, state) = rig(racing())
        val gas = assertNotNull(g.bounds("gas"))
        val shiftUp = assertNotNull(g.bounds(PadIds.RB))
        router.down(1, (gas.left + gas.right) / 2, gas.bottom - 5f, 0)
        router.down(2, (shiftUp.left + shiftUp.right) / 2, (shiftUp.top + shiftUp.bottom) / 2, 0)
        router.setTilt(45f * PI.toFloat() / 180f)
        assertTrue(state.rt > 240, "acelerador a fundo")
        assertEquals(PadButtons.RIGHT_SHOULDER, state.buttons and PadButtons.RIGHT_SHOULDER)
        assertTrue(state.lx > 10_000, "inclinando para a direita, o volante esterça")
    }

    @Test
    fun fpsAtiraEMiraAoToqueEFicaCorrendoComATrava() {
        val l = fps()
        assertTrue(l.widget(PadIds.RT)?.digital == true && l.widget(PadIds.LT)?.digital == true)
        val (g, router, state) = rig(l)
        val rt = assertNotNull(g.bounds(PadIds.RT))
        // Tocar no topo do gatilho já dispara com força total.
        router.down(1, (rt.left + rt.right) / 2, rt.top + 2f, 0)
        assertEquals(255, state.rt)
        router.up(1, 100)
        assertEquals(0, state.rt)

        val sprint = assertNotNull(g.bounds("sprint"))
        val x = (sprint.left + sprint.right) / 2
        val y = (sprint.top + sprint.bottom) / 2
        router.down(2, x, y, 200)
        router.up(2, 240)
        assertEquals(PadButtons.LEFT_THUMB, state.buttons, "correr ficou travado")
        router.down(3, x, y, 600)
        router.up(3, 640)
        assertEquals(0, state.buttons)
    }

    @Test
    fun gatilhoNormalContinuaAnalogico() {
        val (g, router, state) = rig(PadLayouts.default())
        val rt = g.rt
        router.down(1, (rt.left + rt.right) / 2, rt.top + 2f, 0)
        assertTrue(state.rt < 20, "analógico: no topo quase solto")
    }

    @Test
    fun nenhumModeloTemPecaSobrepostaNoCentroDeOutraPrioritaria() {
        // Cada peça visível precisa ser alcançável: tocar no centro dela cai nela ou numa peça de prioridade maior.
        for (preset in PadPreset.entries) {
            val l = PadLayouts.fromPreset(preset, "p", "P", texts)
            val g = GamepadGeometry(w, h, l)
            for (part in g.parts.filter { it.visible && it !is DpadPart }) {
                // O centro do ABXY é o vão entre os botões: confere o botão de baixo.
                val (x, y) = (part as? FacePart)?.buttons?.first()?.shape?.let { it.cx to it.cy }
                    ?: part.bounds.let { (it.left + it.right) / 2 to (it.top + it.bottom) / 2 }
                val hit = g.hit(x, y)
                assertNotNull(hit, "$preset: ${part.id} inalcançável")
            }
        }
    }

    @Test
    fun modosSaoSegurosEPassamPeloArquivo() {
        for (preset in listOf(PadPreset.Racing, PadPreset.Fps)) {
            val l = PadLayouts.fromPreset(preset, "p", "Modo", texts)
            assertTrue(l.widgets.size <= PadLayouts.MAX_WIDGETS)
            assertTrue(l.widgets.all { it.label.length <= PadLayouts.MAX_LABEL })
            assertTrue(l.widgets.all { it.spec.x in 0f..1f && it.spec.y in 0f..1f })
            val back = assertNotNull(LayoutPackage.decode(LayoutPackage.encode(l, null))).layout
            assertEquals(l.widgets, back.widgets, preset.name)
        }
    }

    @Test
    fun modoComVolanteDeArquivoV2VoltaIgual() {
        val l = racing()
        val text = LayoutPackage.encode(l, null)
        assertEquals(l.widgets, assertNotNull(LayoutPackage.decode(text)).layout.widgets)
    }
}
