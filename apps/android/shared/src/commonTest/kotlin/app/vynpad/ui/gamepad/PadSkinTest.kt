package app.vynpad.ui.gamepad

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class PadSkinTest {
    private val w = 2400f
    private val h = 1080f

    // Luminância do corpo dos botões do tema escuro (média de padTop e padBottom).
    private val fill = 0.06f

    private fun uniform(argb: Int, size: Int = 8) = IntArray(size * size) { argb }

    private fun gridOf(argb: Int) = LumaGrid.from(uniform(argb), 8, 8)

    @Test
    fun luminanciaDePretoEBranco() {
        assertEquals(0f, luminance(0xFF000000.toInt()), 1e-6f)
        assertEquals(1f, luminance(0xFFFFFFFF.toInt()), 1e-4f)
    }

    @Test
    fun cinzaMedioSomeNoCorpoDosBotoes() {
        val g = GamepadGeometry(w, h)
        // Fundo cinza médio sem véu: quase a mesma luminância dos botões, ilegível.
        val gray = gridOf(0xFF454545.toInt())
        assertFalse(SkinContrast.isLegible(gray, g, veil = 0f, fillLuma = fill))
    }

    @Test
    fun imagemClaraFicaLegivelSemVeuPoisOsBotoesSaltam() {
        val g = GamepadGeometry(w, h)
        assertTrue(SkinContrast.isLegible(gridOf(0xFFFFFFFF.toInt()), g, veil = 0f, fillLuma = fill))
    }

    @Test
    fun ajusteAutomaticoAumentaOVeuAteFicarLegivel() {
        val g = GamepadGeometry(w, h)
        val gray = gridOf(0xFF454545.toInt())
        val veil = SkinContrast.autoVeil(gray, g, current = 0f, fillLuma = fill)
        assertTrue(veil > 0f && veil <= PadSkins.MAX_VEIL)
        assertTrue(SkinContrast.isLegible(gray, g, veil, fill))
        // ...e é o menor passo: um passo antes ainda não era legível.
        assertFalse(SkinContrast.isLegible(gray, g, veil - 0.05f, fill))
    }

    @Test
    fun ajusteAutomaticoNaoPassaDoMaximo() {
        val g = GamepadGeometry(w, h)
        // Corpo dos botões escuro e fundo igual: nem o véu máximo resolve (luma do fundo ~ 0).
        val v = SkinContrast.autoVeil(gridOf(0xFF000000.toInt()), g, 0f, fillLuma = 0f)
        assertEquals(PadSkins.MAX_VEIL, v)
    }

    @Test
    fun pecaEscondidaNaoEntraNaMedida() {
        val hidden = PadElement.entries.fold(PadLayouts.default()) { l, e -> PadEditing.setVisible(l, e, false) }
        assertNull(SkinContrast.worst(gridOf(0xFF454545.toInt()), GamepadGeometry(w, h, hidden), 0f, fill))
    }

    @Test
    fun gradeMediaSoDaRegiaoPedida() {
        // Metade esquerda branca, metade direita preta.
        val px = IntArray(16 * 16) { if (it % 16 < 8) 0xFFFFFFFF.toInt() else 0xFF000000.toInt() }
        val grid = LumaGrid.from(px, 16, 16, cols = 8, rows = 8)
        assertEquals(1f, grid.mean(0f, 0f, 0.4f, 1f), 1e-3f)
        assertEquals(0f, grid.mean(0.6f, 0f, 1f, 1f), 1e-3f)
    }

    @Test
    fun corDominanteEAVermelhaMesmoComFundoCinza() {
        val px = IntArray(100) { if (it < 30) 0xFFE02020.toInt() else 0xFF808080.toInt() }
        val accent = assertNotNull(AccentPicker.pick(px))
        val r = ((accent shr 16) and 0xFF).toInt()
        val g = ((accent shr 8) and 0xFF).toInt()
        assertTrue(r > 180 && g < 90, "r=$r g=$g")
    }

    @Test
    fun imagemCinzaNaoTemCorMarcante() {
        assertNull(AccentPicker.pick(uniform(0xFF909090.toInt())))
    }

    @Test
    fun corMuitoEscuraEOuLavadaEAjustadaParaDestaque() {
        // Azul-escuro: o destaque sai mais claro e saturado para ser visível no vidro.
        val accent = assertNotNull(AccentPicker.pick(uniform(0xFF202878.toInt())))
        val b = (accent and 0xFF).toInt()
        assertTrue(b >= 0xB0, "b=$b")
    }

    @Test
    fun tonsDiferentesVencemPelaMaiorMassa() {
        val px = IntArray(100) { if (it < 70) 0xFF22C060.toInt() else 0xFFE02020.toInt() }
        val accent = assertNotNull(AccentPicker.pick(px))
        val g = ((accent shr 8) and 0xFF).toInt()
        val r = ((accent shr 16) and 0xFF).toInt()
        assertTrue(g > r, "r=$r g=$g")
    }

    @Test
    fun layoutComSkinIdaEVoltaPeloJson() {
        val skin = PadSkin(image = "s1", veil = 0.6f, blur = true, accent = 0xFFE02020L, padOpacity = 0.8f)
        val layout = PadLayouts.default().copy(skin = skin)
        val s = app.vynpad.settings.AppSettings().withLayout(layout)
        assertEquals(s, app.vynpad.settings.AppSettings.decode(app.vynpad.settings.AppSettings.encode(s)))
    }
}
