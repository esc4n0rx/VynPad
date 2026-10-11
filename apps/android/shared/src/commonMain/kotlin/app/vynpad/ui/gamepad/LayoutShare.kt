package app.vynpad.ui.gamepad

import kotlin.io.encoding.Base64
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

/**
 * O arquivo de um layout compartilhado (`.vynlayout`): o layout, sem o nome do arquivo da imagem
 * (que só existe neste aparelho), e a imagem de fundo embutida em Base64 (JPEG).
 */
@Serializable
data class LayoutPackage(
    val format: String = FORMAT,
    val version: Int = VERSION,
    @Serializable(with = PadLayoutSerializer::class) val layout: PadLayout,
    val image: String? = null,
) {
    companion object {
        const val FORMAT = "vynpad-layout"

        /** 2: o layout é uma lista de widgets (o 1 tinha as 11 peças fixas e é migrado ao ler). */
        const val VERSION = 2

        /** Acima disto o arquivo é recusado antes de ser lido (a imagem já vem reduzida). */
        const val MAX_BYTES = 12 * 1024 * 1024

        private val json = Json {
            ignoreUnknownKeys = true
            coerceInputValues = true
            encodeDefaults = true
        }

        fun encode(layout: PadLayout, image: ByteArray?): String {
            val pkg = LayoutPackage(
                layout = layout.copy(skin = layout.skin.copy(image = null)),
                image = image?.let { Base64.encode(it) },
            )
            return json.encodeToString(serializer(), pkg)
        }

        /** Lê um arquivo de fora; `null` se não for um layout do VynPad que este app entenda. */
        fun decode(text: String): LayoutPackage? {
            if (text.length > MAX_BYTES) return null
            val pkg = runCatching { json.decodeFromString(serializer(), text) }.getOrNull() ?: return null
            if (pkg.format != FORMAT || pkg.version !in 1..VERSION) return null
            return pkg.copy(layout = pkg.layout.sanitized())
        }

        /** A imagem embutida, ou `null` se não há ou o Base64 está quebrado. */
        fun imageBytes(pkg: LayoutPackage): ByteArray? = pkg.image?.let { runCatching { Base64.decode(it) }.getOrNull() }?.takeIf { it.isNotEmpty() }
    }
}

/**
 * Um layout vindo de um arquivo é dado de fora: nada dele pode tirar peça da tela, estourar
 * limites ou carregar um nome gigante. Tudo é trazido de volta à faixa válida.
 */
fun PadLayout.sanitized(): PadLayout {
    fun f(v: Float, lo: Float, hi: Float, fallback: Float) = if (v.isNaN()) fallback else v.coerceIn(lo, hi)
    fun stick(s: StickConfig) = s.copy(
        travelScale = f(s.travelScale, PadLayouts.MIN_TRAVEL, PadLayouts.MAX_TRAVEL, 1f),
        deadzone = f(s.deadzone, 0f, PadLayouts.MAX_DEADZONE, 0f),
    )
    val clean = widgets.distinctBy { it.id }.take(PadLayouts.MAX_WIDGETS).map { w ->
        w.copy(
            id = w.id.take(MAX_ID),
            spec = w.spec.copy(
                x = f(w.spec.x, 0f, 1f, 0.5f),
                y = f(w.spec.y, 0f, 1f, 0.5f),
                scale = f(w.spec.scale, PadLayouts.MIN_SCALE, PadLayouts.MAX_SCALE, 1f),
                opacity = f(w.spec.opacity, PadLayouts.MIN_OPACITY, 1f, 1f),
            ),
            label = w.label.take(PadLayouts.MAX_LABEL),
            // Uma máscara fora da lista de botões (um arquivo malicioso poderia apertar tudo) não aciona nada.
            mask = if (w.kind == WidgetKind.Button && w.mask in PadLayouts.ButtonMasks) w.mask else 0,
            stick = stick(w.stick),
        )
    }
    return copy(
        name = name.trim().take(MAX_NAME).ifEmpty { "Layout" },
        // Um arquivo sem nenhuma peça deixaria a tela vazia: volta ao controle padrão.
        widgets = clean.ifEmpty { PadLayouts.DefaultWidgets },
        skin = skin.copy(
            veil = f(skin.veil, 0f, PadSkins.MAX_VEIL, PadSkins.DEFAULT_VEIL),
            padOpacity = f(skin.padOpacity, PadSkins.MIN_PAD_OPACITY, 1f, 1f),
            // O nome do arquivo de imagem é local: quem importa define o seu.
            image = null,
        ),
    )
}

private const val MAX_NAME = 24
private const val MAX_ID = 32
