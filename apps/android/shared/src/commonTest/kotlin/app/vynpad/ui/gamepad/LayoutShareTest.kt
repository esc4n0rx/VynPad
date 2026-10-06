package app.vynpad.ui.gamepad

import app.vynpad.settings.AppSettings
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class LayoutShareTest {
    private val custom = PadLayouts.default("Elden Ring")
        .with(PadElement.Dpad, ElementSpec(0.4f, 0.6f, scale = 1.2f, opacity = 0.7f))
        .withStick(true, StickConfig(floating = false, travelScale = 1.3f, deadzone = 0.1f))
        .copy(skin = PadSkin(image = "so-local", veil = 0.6f, accent = 0xFFE02020L, padOpacity = 0.8f))

    @Test
    fun idaEVoltaPreservaOLayoutMenosONomeDoArquivoLocal() {
        val pkg = assertNotNull(LayoutPackage.decode(LayoutPackage.encode(custom, byteArrayOf(1, 2, 3))))
        assertEquals(custom.copy(skin = custom.skin.copy(image = null)), pkg.layout.copy(id = custom.id))
        assertContentEquals(byteArrayOf(1, 2, 3), LayoutPackage.imageBytes(pkg))
    }

    @Test
    fun semImagemOPacoteFicaSemImagem() {
        val pkg = assertNotNull(LayoutPackage.decode(LayoutPackage.encode(custom, null)))
        assertNull(LayoutPackage.imageBytes(pkg))
    }

    @Test
    fun arquivoDeOutroFormatoOuVersaoNovaERecusado() {
        assertNull(LayoutPackage.decode("não é json"))
        assertNull(LayoutPackage.decode("""{"format":"outra-coisa","version":1,"layout":{"id":"a","name":"b"}}"""))
        assertNull(LayoutPackage.decode("""{"format":"vynpad-layout","version":99,"layout":{"id":"a","name":"b"}}"""))
        assertNull(LayoutPackage.decode("""{"format":"vynpad-layout","version":1}"""))
    }

    @Test
    fun valoresAbsurdosVoltamParaAFaixaValida() {
        val text = """{"format":"vynpad-layout","version":1,"layout":{"id":"x","name":"${"n".repeat(500)}",
            "elements":{"Dpad":{"x":9,"y":-4,"scale":50,"opacity":0}},
            "leftStick":{"travelScale":99,"deadzone":5},
            "skin":{"veil":7,"padOpacity":-1,"image":"../../etc/passwd"}}}"""
        val l = assertNotNull(LayoutPackage.decode(text)).layout
        val d = l.spec(PadElement.Dpad)
        assertEquals(1f, d.x)
        assertEquals(0f, d.y)
        assertEquals(PadLayouts.MAX_SCALE, d.scale)
        assertEquals(PadLayouts.MIN_OPACITY, d.opacity)
        assertEquals(PadLayouts.MAX_TRAVEL, l.leftStick.travelScale)
        assertEquals(PadLayouts.MAX_DEADZONE, l.leftStick.deadzone)
        assertEquals(PadSkins.MAX_VEIL, l.skin.veil)
        assertEquals(PadSkins.MIN_PAD_OPACITY, l.skin.padOpacity)
        assertEquals(24, l.name.length)
        // Um arquivo de fora nunca escolhe o caminho da imagem no aparelho.
        assertNull(l.skin.image)
    }

    @Test
    fun layoutImportadoCompletaPecasFaltantes() {
        val l = assertNotNull(
            LayoutPackage.decode("""{"format":"vynpad-layout","version":1,"layout":{"id":"x","name":"Mini"}}"""),
        ).layout
        assertEquals(PadElement.entries.toSet(), l.elements.keys)
    }

    @Test
    fun base64QuebradoNaoDerrubaAImportacao() {
        val pkg = assertNotNull(
            LayoutPackage.decode("""{"format":"vynpad-layout","version":1,"layout":{"id":"x","name":"a"},"image":"@@@"}"""),
        )
        assertNull(LayoutPackage.imageBytes(pkg))
    }

    @Test
    fun arquivoGrandeDemaisERecusadoSemSerLido() {
        assertNull(LayoutPackage.decode("x".repeat(LayoutPackage.MAX_BYTES + 1)))
    }

    @Test
    fun adicionarImportadoCriaIdENomeNovosEAtiva() {
        val base = AppSettings()
        val added = base.addImported(custom.copy(id = "do-arquivo"))
        assertEquals(2, added.layouts.size)
        assertEquals(added.activeLayoutId, added.activeLayout.id)
        assertTrue(added.activeLayout.id != "do-arquivo" && added.activeLayout.id != PadLayouts.DefaultId)
        // Importar o mesmo nome de novo não duplica o nome.
        val twice = added.addImported(custom)
        assertEquals(twice.layouts.size, twice.layouts.map { it.name }.toSet().size)
    }
}
