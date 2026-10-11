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
        .with(PadIds.Dpad, ElementSpec(0.4f, 0.6f, scale = 1.2f, opacity = 0.7f))
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
        val d = l.spec(PadIds.Dpad)
        assertEquals(1f, d.x)
        assertEquals(0f, d.y)
        assertEquals(PadLayouts.MAX_SCALE, d.scale)
        assertEquals(PadLayouts.MIN_OPACITY, d.opacity)
        assertEquals(PadLayouts.MAX_TRAVEL, l.stick(true).travelScale)
        assertEquals(PadLayouts.MAX_DEADZONE, l.stick(true).deadzone)
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
        assertEquals(PadIds.BuiltIn.toSet(), l.widgets.map { it.id }.toSet())
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

    @Test
    fun arquivoDaVersao1MigraParaWidgetsSemPerderOsAjustes() {
        val text = """{"format":"vynpad-layout","version":1,"layout":{"id":"x","name":"Velho",
            "elements":{"Face":{"x":0.7,"y":0.5,"scale":1.2,"visible":false},"RT":{"x":0.8,"y":0.2}},
            "rightStick":{"floating":false,"deadzone":0.2}}}"""
        val l = assertNotNull(LayoutPackage.decode(text)).layout
        assertEquals(PadLayouts.DefaultWidgets.map { it.id }, l.widgets.map { it.id })
        assertEquals(ElementSpec(0.7f, 0.5f, scale = 1.2f, visible = false), l.spec(PadIds.Face))
        assertEquals(0.8f, l.spec(PadIds.RT).x)
        assertEquals(StickConfig(floating = false, deadzone = 0.2f), l.stick(false))
        assertEquals(StickConfig(), l.stick(true))
        // O que o arquivo não mencionava fica como no controle padrão.
        assertEquals(PadLayouts.DefaultSpecs.getValue(PadIds.Dpad), l.spec(PadIds.Dpad))
    }

    @Test
    fun layoutNovoIdaEVoltaPeloArquivo() {
        val layout = PadLayouts.default().copy(
            widgets = PadLayouts.DefaultWidgets + PadWidget(
                "extra",
                WidgetKind.Button,
                ElementSpec(0.5f, 0.6f),
                mask = 0x1000,
                label = "OK",
            ),
        )
        val back = assertNotNull(LayoutPackage.decode(LayoutPackage.encode(layout, null))).layout
        assertEquals(layout.widgets, back.widgets)
    }

    @Test
    fun pecasDeArquivoSaoLimitadasEDeduplicadas() {
        val many = (0 until PadLayouts.MAX_WIDGETS + 20).map { PadWidget("w$it", WidgetKind.Button, ElementSpec(0.5f, 0.5f)) }
        val l = PadLayouts.default().copy(widgets = many + many.first().copy(label = "repetida")).sanitized()
        assertEquals(PadLayouts.MAX_WIDGETS, l.widgets.size)
        assertEquals(l.widgets.size, l.widgets.map { it.id }.toSet().size)
        assertEquals(PadLayouts.DefaultWidgets, PadLayouts.default().copy(widgets = emptyList()).sanitized().widgets)
    }
}
