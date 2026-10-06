package app.vynpad.settings

import androidx.compose.runtime.Composable
import app.vynpad.input.RumbleSettings
import app.vynpad.input.RumbleStrength
import app.vynpad.ui.gamepad.PadLayout
import app.vynpad.ui.gamepad.PadLayouts
import app.vynpad.ui.theme.ThemeMode
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

/** Onde os ajustes ficam guardados (um texto só); cada plataforma decide o meio. */
interface SettingsStore {
    fun load(): String?

    fun save(json: String)
}

@Composable
expect fun rememberSettingsStore(): SettingsStore

/**
 * Tudo o que o usuário ajusta no app e deve sobreviver ao fechamento: tema, vibrações e os
 * layouts do controle. Cada campo tem padrão, então um arquivo antigo ou parcial continua válido.
 */
@Serializable
data class AppSettings(
    val theme: ThemeMode = ThemeMode.Dark,
    val hapticsEnabled: Boolean = true,
    val rumbleEnabled: Boolean = true,
    val rumbleStrength: RumbleStrength = RumbleStrength.Strong,
    val layouts: List<PadLayout> = listOf(PadLayouts.default()),
    val activeLayoutId: String = PadLayouts.DefaultId,
) {
    val rumble: RumbleSettings get() = RumbleSettings(rumbleEnabled, rumbleStrength)

    /** O layout em uso; se o id sumiu, o primeiro (sempre há ao menos um). */
    val activeLayout: PadLayout get() = layouts.firstOrNull { it.id == activeLayoutId } ?: layouts.first()

    fun withRumble(r: RumbleSettings) = copy(rumbleEnabled = r.enabled, rumbleStrength = r.strength)

    /** Troca o layout de mesmo `id` pelo editado. */
    fun withLayout(layout: PadLayout) = copy(layouts = layouts.map { if (it.id == layout.id) layout else it })

    fun select(id: String) = if (layouts.any { it.id == id }) copy(activeLayoutId = id) else this

    /** Cria um layout novo a partir de [source], o torna ativo e dá a ele um nome livre. */
    fun duplicate(source: PadLayout): AppSettings {
        var n = layouts.size + 1
        while (layouts.any { it.name == "Layout $n" }) n++
        val copy = source.copy(id = newId(), name = "Layout $n")
        return copy(layouts = layouts + copy, activeLayoutId = copy.id)
    }

    fun rename(id: String, name: String): AppSettings {
        val clean = name.trim().take(MAX_NAME)
        return if (clean.isEmpty()) this else copy(layouts = layouts.map { if (it.id == id) it.copy(name = clean) else it })
    }

    /** Remove um layout; o último que sobrar não pode ser removido. */
    fun remove(id: String): AppSettings {
        if (layouts.size <= 1) return this
        val rest = layouts.filter { it.id != id }
        return copy(layouts = rest, activeLayoutId = if (activeLayoutId == id) rest.first().id else activeLayoutId)
    }

    private fun newId(): String {
        var id: String
        do {
            id = "l" + (1..ID_LENGTH).map { ID_ALPHABET.random() }.joinToString("")
        } while (layouts.any { it.id == id })
        return id
    }

    companion object {
        const val MAX_NAME = 24
        private const val ID_LENGTH = 8
        private const val ID_ALPHABET = "abcdefghijklmnopqrstuvwxyz0123456789"

        private val json = Json {
            ignoreUnknownKeys = true
            coerceInputValues = true
            encodeDefaults = true
        }

        fun encode(settings: AppSettings): String = json.encodeToString(serializer(), settings)

        /** Lê o que foi guardado; texto ausente ou corrompido vira os padrões (nunca falha). */
        fun decode(text: String?): AppSettings {
            val parsed = text?.let { runCatching { json.decodeFromString(serializer(), it) }.getOrNull() } ?: return AppSettings()
            // Garante a invariante: ao menos um layout.
            return if (parsed.layouts.isEmpty()) parsed.copy(layouts = listOf(PadLayouts.default())) else parsed
        }
    }
}
