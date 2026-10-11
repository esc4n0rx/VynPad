package app.vynpad.ui.gamepad

import app.vynpad.input.PadButtons
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class PadLayoutTest {
    private val w = 2400f
    private val h = 1080f
    private val aspect = w / h

    @Test
    fun padraoReproduzOsNumerosOriginais() {
        val g = GamepadGeometry(w, h)
        assertEquals(0.14f * w, g.leftStickZone.cx, 0.01f)
        assertEquals(0.25f * h, g.leftStickZone.r, 0.01f)
        assertEquals(0.03f * w, g.lt.left, 0.01f)
        assertEquals(0.04f * h, g.lt.top, 0.01f)
        assertEquals(0.97f * w, g.rt.right, 0.01f)
        assertEquals(0.20f * h, g.rt.bottom, 0.01f)
        assertEquals(0.86f * w, g.faceCenter.cx, 0.01f)
    }

    @Test
    fun moverLevaAAreaDeToqueJunto() {
        val layout = PadLayouts.default().with(PadIds.Dpad, ElementSpec(0.5f, 0.5f))
        val g = GamepadGeometry(w, h, layout)
        assertEquals(0.5f * w, g.dpad.cx, 0.01f)
        assertTrue(g.hit(0.5f * w, 0.5f * h + g.dpad.r * 0.7f) is Hit.Dpad)
    }

    @Test
    fun escalaMudaOTamanhoEOCursoDoAnalogico() {
        val layout = PadLayouts.default().with(PadIds.LeftStick, ElementSpec(0.14f, 0.47f, scale = 1.5f))
        val g = GamepadGeometry(w, h, layout)
        assertEquals(0.25f * h * 1.5f, g.leftStickZone.r, 0.01f)
        assertEquals(g.stickTravel * 1.5f, g.travelOf(true), 0.01f)
        assertEquals(g.stickTravel, g.travelOf(false), 0.01f)
    }

    @Test
    fun pecaEscondidaNaoRecebeToque() {
        val layout = PadLayouts.default().with(PadIds.Face, ElementSpec(0.86f, 0.46f, visible = false))
        val g = GamepadGeometry(w, h, layout)
        val a = g.faceButtons.first { it.mask == PadButtons.A }.shape
        assertNull(g.hit(a.cx, a.cy))
        // ...mas o editor ainda a encontra, para o usuário poder reexibi-la.
        assertEquals(PadIds.Face, g.elementAt(a.cx, a.cy))
    }

    @Test
    fun editorEscolheAPecaMenorNaSobreposicao() {
        val g = GamepadGeometry(w, h)
        val guide = g.guide.shape
        assertEquals(PadIds.Guide, g.elementAt(guide.cx, guide.cy))
    }

    @Test
    fun ladoEmBrancoNaoSelecionaNada() {
        assertNull(GamepadGeometry(w, h).elementAt(0.5f * w, 0.95f * h))
    }

    @Test
    fun moverPrendeNaTelaEAplicaOImaNaGrade() {
        val base = PadLayouts.default()
        val free = PadEditing.moveTo(base, PadIds.Dpad, 0.5037f, 0.5013f, aspect, snap = false)
        assertEquals(0.5037f, free.spec(PadIds.Dpad).x, 1e-5f)
        val snapped = PadEditing.moveTo(base, PadIds.Dpad, 0.5037f, 0.5013f, aspect, snap = true)
        // y cai na grade de 5 % da altura.
        assertEquals(0.5f, snapped.spec(PadIds.Dpad).y, 1e-5f)
        val out = PadEditing.moveTo(base, PadIds.Dpad, -3f, 9f, aspect, snap = false)
        assertTrue(out.spec(PadIds.Dpad).x > 0f && out.spec(PadIds.Dpad).y < 1f)
    }

    @Test
    fun imaNaGradeTemAMesmaDistanciaEmPixelsNosDoisEixos() {
        val snapped = PadEditing.moveTo(PadLayouts.default(), PadIds.Dpad, 0.3f, 0.3f, aspect, snap = true)
        val pxX = snapped.spec(PadIds.Dpad).x * w
        val stepPx = PadEditing.GRID * h
        assertEquals(0f, (pxX / stepPx) - kotlin.math.round(pxX / stepPx), 1e-3f)
    }

    @Test
    fun escalaEOpacidadeFicamNosLimites() {
        val l = PadEditing.setScale(PadLayouts.default(), PadIds.Face, 9f)
        assertEquals(PadLayouts.MAX_SCALE, l.spec(PadIds.Face).scale)
        assertEquals(PadLayouts.MIN_SCALE, PadEditing.setScale(l, PadIds.Face, 0f).spec(PadIds.Face).scale)
        assertEquals(PadLayouts.MIN_OPACITY, PadEditing.setOpacity(l, PadIds.Face, 0f).spec(PadIds.Face).opacity)
    }

    @Test
    fun restaurarUmaPecaVoltaAoPadraoSemMexerNasOutras() {
        val moved = PadEditing.moveTo(PadLayouts.default(), PadIds.Dpad, 0.5f, 0.5f, aspect, false)
            .let { PadEditing.moveTo(it, PadIds.Face, 0.4f, 0.4f, aspect, false) }
        val back = PadEditing.resetElement(moved, PadIds.Dpad)
        assertEquals(PadLayouts.DefaultSpecs.getValue(PadIds.Dpad), back.spec(PadIds.Dpad))
        assertEquals(0.4f, back.spec(PadIds.Face).x, 1e-5f)
    }

    @Test
    fun todoModeloTemTodasAsPecasDentroDaTela() {
        for (preset in PadPreset.entries) {
            val l = PadLayouts.fromPreset(preset, "p", "P")
            assertEquals(PadIds.BuiltIn.toSet(), l.widgets.map { it.id }.toSet(), preset.name)
            for ((el, s) in l.widgets.map { it.id to it.spec }) {
                assertTrue(s.x in 0f..1f && s.y in 0f..1f, "$preset $el")
                assertTrue(s.scale in PadLayouts.MIN_SCALE..PadLayouts.MAX_SCALE, "$preset $el escala")
            }
        }
    }

    @Test
    fun canhotoEspelhaAsPosicoes() {
        val l = PadLayouts.fromPreset(PadPreset.LeftHanded, "p", "P")
        assertEquals(1f - 0.86f, l.spec(PadIds.Face).x, 1e-5f)
        assertEquals(0.5f, l.spec(PadIds.Guide).x, 1e-5f)
    }

    @Test
    fun soDpadEscondeAnalogicosEGatilhosMasMantemODirecional() {
        val l = PadLayouts.fromPreset(PadPreset.DpadOnly, "p", "P")
        val hidden = PadIds.BuiltIn.filter { !l.spec(it).visible }.toSet()
        assertEquals(setOf(PadIds.LeftStick, PadIds.RightStick, PadIds.LT, PadIds.RT), hidden)
        assertTrue(l.spec(PadIds.Dpad).visible)
        assertNotNull(GamepadGeometry(w, h, l).hit(0.20f * w, 0.60f * h - 5f))
    }
}
