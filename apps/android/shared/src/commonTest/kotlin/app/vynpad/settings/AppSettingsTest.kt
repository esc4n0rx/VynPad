package app.vynpad.settings

import app.vynpad.input.RumbleStrength
import app.vynpad.ui.gamepad.PadIds
import app.vynpad.ui.gamepad.PadLayouts
import app.vynpad.ui.theme.ThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class AppSettingsTest {
    @Test
    fun ida_e_volta_preserva_tudo() {
        val base = AppSettings(theme = ThemeMode.Light, rumbleStrength = RumbleStrength.Soft, hapticsEnabled = false)
        val moved = base.activeLayout.with(PadIds.Dpad, base.activeLayout.spec(PadIds.Dpad).copy(x = 0.4f, opacity = 0.5f))
        val s = base.withLayout(moved)
        assertEquals(s, AppSettings.decode(AppSettings.encode(s)))
    }

    @Test
    fun texto_ausente_ou_quebrado_vira_padrao() {
        assertEquals(AppSettings(), AppSettings.decode(null))
        assertEquals(AppSettings(), AppSettings.decode("{isto não é json"))
    }

    @Test
    fun arquivo_parcial_e_chaves_desconhecidas_sao_aceitos() {
        val s = AppSettings.decode("""{"hapticsEnabled":false,"campoNovoDoFuturo":1}""")
        assertFalse(s.hapticsEnabled)
        assertEquals(ThemeMode.Dark, s.theme)
        assertEquals(1, s.layouts.size)
    }

    @Test
    fun layout_antigo_sem_todas_as_pecas_completa_com_o_padrao() {
        val s = AppSettings.decode("""{"layouts":[{"id":"x","name":"Velho","elements":{"Dpad":{"x":0.5,"y":0.5}}}],"activeLayoutId":"x"}""")
        assertEquals(0.5f, s.activeLayout.spec(PadIds.Dpad).x)
        assertEquals(PadLayouts.DefaultSpecs.getValue(PadIds.Face), s.activeLayout.spec(PadIds.Face))
    }

    @Test
    fun lista_de_layouts_vazia_volta_a_ter_o_padrao() {
        assertEquals(1, AppSettings.decode("""{"layouts":[]}""").layouts.size)
    }

    @Test
    fun duplicar_cria_layout_ativo_com_id_e_nome_novos() {
        val s = AppSettings().duplicate(AppSettings().activeLayout)
        assertEquals(2, s.layouts.size)
        assertNotEquals(PadLayouts.DefaultId, s.activeLayoutId)
        assertEquals(s.activeLayoutId, s.activeLayout.id)
        assertEquals(2, s.layouts.map { it.id }.toSet().size)
    }

    @Test
    fun remover_o_ativo_seleciona_outro_e_o_ultimo_nao_sai() {
        val two = AppSettings().duplicate(AppSettings().activeLayout)
        val back = two.remove(two.activeLayoutId)
        assertEquals(listOf(PadLayouts.DefaultId), back.layouts.map { it.id })
        assertEquals(PadLayouts.DefaultId, back.activeLayoutId)
        assertEquals(back, back.remove(PadLayouts.DefaultId))
    }

    @Test
    fun renomear_ignora_nome_vazio_e_limita_o_tamanho() {
        val s = AppSettings()
        assertEquals(s, s.rename(PadLayouts.DefaultId, "   "))
        val long = s.rename(PadLayouts.DefaultId, "x".repeat(100))
        assertEquals(AppSettings.MAX_NAME, long.activeLayout.name.length)
        assertTrue(s.rename(PadLayouts.DefaultId, "  Elden Ring ").activeLayout.name == "Elden Ring")
    }

    @Test
    fun selecionar_id_inexistente_nao_muda_nada() {
        val s = AppSettings()
        assertEquals(s, s.select("nao-existe"))
    }
}
