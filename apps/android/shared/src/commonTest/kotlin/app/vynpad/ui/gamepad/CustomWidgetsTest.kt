package app.vynpad.ui.gamepad

import app.vynpad.input.GamepadState
import app.vynpad.input.PadButtons
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

/** Peças que o usuário acrescenta no editor: botão livre (com trava), pedal e as operações de edição. */
class CustomWidgetsTest {
    private val w = 2400f
    private val h = 1080f

    private fun rig(layout: PadLayout): Triple<GamepadGeometry, TouchRouter, GamepadState> {
        val g = GamepadGeometry(w, h, layout)
        val state = GamepadState()
        return Triple(g, TouchRouter({ g }, state, PadVisuals()), state)
    }

    private fun withWidget(template: PadWidget): Pair<PadLayout, String> = assertNotNull(PadEditing.add(PadLayouts.default(), template))

    @Test
    fun adicionarDaIdLivreEUnico() {
        val (one, id1) = withWidget(PadLayouts.newButton())
        val (two, id2) = assertNotNull(PadEditing.add(one, PadLayouts.newButton()))
        assertTrue(id1 != id2)
        assertEquals(two.widgets.size, two.widgets.map { it.id }.toSet().size)
        // Remover e adicionar de novo reaproveita o id livre sem colidir com os que ficaram.
        val again = assertNotNull(PadEditing.add(PadEditing.remove(two, id1), PadLayouts.newButton()))
        assertEquals(two.widgets.size, again.first.widgets.size)
        assertEquals(again.first.widgets.size, again.first.widgets.map { it.id }.toSet().size)
    }

    @Test
    fun limiteDePecasImpedeAdicionar() {
        var layout = PadLayouts.default()
        while (layout.widgets.size < PadLayouts.MAX_WIDGETS) layout = assertNotNull(PadEditing.add(layout, PadLayouts.newButton())).first
        assertNull(PadEditing.add(layout, PadLayouts.newButton()))
        assertNull(PadEditing.duplicate(layout, PadIds.Dpad))
    }

    @Test
    fun duplicarDesloca() {
        val (layout, id) = withWidget(PadLayouts.newButton().copy(spec = ElementSpec(0.5f, 0.5f), mask = PadButtons.X, label = "Ok"))
        val (twice, copy) = assertNotNull(PadEditing.duplicate(layout, id))
        assertEquals(PadButtons.X, twice.widget(copy)?.mask)
        assertEquals("Ok", twice.widget(copy)?.label)
        assertTrue(twice.spec(copy).x > 0.5f && twice.spec(copy).y > 0.5f)
    }

    @Test
    fun removerTiraAPecaEOGeometriaNaoAEnxerga() {
        val (layout, id) = withWidget(PadLayouts.newButton().copy(spec = ElementSpec(0.5f, 0.5f)))
        assertNotNull(GamepadGeometry(w, h, layout).bounds(id))
        val removed = PadEditing.remove(layout, id)
        assertNull(GamepadGeometry(w, h, removed).bounds(id))
        // Uma peça padrão removida não quebra a geometria (os atalhos caem no desenho original).
        val noDpad = PadEditing.remove(PadLayouts.default(), PadIds.Dpad)
        assertNotNull(GamepadGeometry(w, h, noDpad).dpad)
    }

    @Test
    fun botaoLivreAcionaAMascaraEscolhida() {
        val (layout, id) = withWidget(PadLayouts.newButton().copy(spec = ElementSpec(0.5f, 0.5f), mask = PadButtons.DPAD_UP))
        val (g, router, state) = rig(layout)
        val b = assertNotNull(g.bounds(id))
        router.down(1, (b.left + b.right) / 2, (b.top + b.bottom) / 2, 0)
        assertEquals(PadButtons.DPAD_UP, state.buttons)
        router.up(1, 50)
        assertEquals(0, state.buttons)
    }

    @Test
    fun botaoQueTravaAlternaACadaToque() {
        val (layout, id) = withWidget(PadLayouts.newButton().copy(spec = ElementSpec(0.5f, 0.5f), mask = PadButtons.B, latch = true))
        val (g, router, state) = rig(layout)
        val b = assertNotNull(g.bounds(id))
        val x = (b.left + b.right) / 2
        val y = (b.top + b.bottom) / 2
        router.down(1, x, y, 0)
        router.up(1, 40)
        assertEquals(PadButtons.B, state.buttons, "ficou apertado depois de soltar o dedo")
        router.down(2, x, y, 500)
        router.up(2, 540)
        assertEquals(0, state.buttons, "o segundo toque soltou")
    }

    @Test
    fun botaoQueTravaSomaComOutrosToques() {
        val (layout, id) = withWidget(PadLayouts.newButton().copy(spec = ElementSpec(0.5f, 0.5f), mask = PadButtons.B, latch = true))
        val (g, router, state) = rig(layout)
        val b = assertNotNull(g.bounds(id))
        router.down(1, (b.left + b.right) / 2, (b.top + b.bottom) / 2, 0)
        router.up(1, 40)
        val a = g.faceButtons.first { it.mask == PadButtons.A }.shape
        router.down(2, a.cx, a.cy, 100)
        assertEquals(PadButtons.B or PadButtons.A, state.buttons)
    }

    @Test
    fun cancelarTudoSoltaOQueEstavaTravado() {
        val (layout, id) = withWidget(PadLayouts.newButton().copy(spec = ElementSpec(0.5f, 0.5f), mask = PadButtons.Y, latch = true))
        val (g, router, state) = rig(layout)
        val b = assertNotNull(g.bounds(id))
        router.down(1, (b.left + b.right) / 2, (b.top + b.bottom) / 2, 0)
        router.up(1, 40)
        router.cancelAll()
        assertEquals(0, state.buttons)
    }

    @Test
    fun pedalVaiDeZeroACemPelaAlturaInteira() {
        val (layout, id) = withWidget(PadLayouts.newPedal(left = false))
        val (g, router, state) = rig(layout)
        val b = assertNotNull(g.bounds(id))
        val x = (b.left + b.right) / 2
        router.down(1, x, b.top + 1f, 0)
        assertTrue(state.rt in 0..5, "no topo o pedal está solto")
        router.move(1, x, (b.top + b.bottom) / 2)
        assertTrue(state.rt in 120..135, "no meio vale cerca de metade (${state.rt})")
        router.move(1, x, b.bottom)
        assertEquals(255, state.rt)
        assertEquals(0, state.lt)
        router.up(1, 100)
        assertEquals(0, state.rt)
    }

    @Test
    fun pedalEsquerdoVaiParaLT() {
        val (layout, id) = withWidget(PadLayouts.newPedal(left = true).copy(spec = ElementSpec(0.4f, 0.55f)))
        val (g, router, state) = rig(layout)
        val b = assertNotNull(g.bounds(id))
        router.down(1, (b.left + b.right) / 2, b.bottom, 0)
        assertEquals(255, state.lt)
    }

    @Test
    fun ajustesDeBotaoFicamNosLimites() {
        val (layout, id) = withWidget(PadLayouts.newButton())
        val l = PadEditing.setLabel(layout, id, "x".repeat(50))
        assertEquals(PadLayouts.MAX_LABEL, l.widget(id)?.label?.length)
        assertEquals(PadButtons.RIGHT_SHOULDER, PadEditing.setMask(l, id, PadButtons.RIGHT_SHOULDER).widget(id)?.mask)
        assertEquals(false, PadEditing.setSide(withWidget(PadLayouts.newPedal(true)).first, "w1", false).widget("w1")?.left)
    }

    @Test
    fun arquivoNaoAcionaMascaraForaDaLista() {
        val bad = PadLayouts.default().copy(
            widgets = PadLayouts.DefaultWidgets + PadWidget("evil", WidgetKind.Button, ElementSpec(0.5f, 0.5f), mask = 0xFFFF),
        ).sanitized()
        assertEquals(0, bad.widget("evil")?.mask)
        assertEquals(PadButtons.GUIDE, bad.widget(PadIds.Guide)?.mask)
    }

    @Test
    fun pecasNovasIdaEVoltaPeloArquivoComTravaEPedal() {
        val base = PadLayouts.default()
        val (a, _) = assertNotNull(PadEditing.add(base, PadLayouts.newButton().copy(latch = true, label = "FM")))
        val (b, _) = assertNotNull(PadEditing.add(a, PadLayouts.newPedal(left = false)))
        val back = assertNotNull(LayoutPackage.decode(LayoutPackage.encode(b, null))).layout
        assertEquals(b.widgets, back.widgets)
    }
}
