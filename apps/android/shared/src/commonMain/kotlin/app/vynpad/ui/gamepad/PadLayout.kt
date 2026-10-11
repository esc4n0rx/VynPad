package app.vynpad.ui.gamepad

import app.vynpad.input.PadButtons
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonTransformingSerializer

/** Os ids das peças do controle padrão (as 11 do Xbox). Um layout pode ter outras, com ids próprios. */
object PadIds {
    const val LeftStick = "left-stick"
    const val RightStick = "right-stick"
    const val Dpad = "dpad"

    /** Os quatro botões ABXY, movidos juntos. */
    const val Face = "face"
    const val Back = "back"
    const val Guide = "guide"
    const val Start = "start"
    const val LT = "lt"
    const val LB = "lb"
    const val RB = "rb"
    const val RT = "rt"

    /** Ordem de desenho do controle padrão. */
    val BuiltIn: List<String> = listOf(LT, RT, LB, RB, Back, Guide, Start, Dpad, Face, LeftStick, RightStick)

    /** Como o formato antigo (v1, um `enum` fixo) chamava cada peça. */
    internal val LegacyNames: Map<String, String> = mapOf(
        LeftStick to "LeftStick",
        RightStick to "RightStick",
        Dpad to "Dpad",
        Face to "Face",
        Back to "Back",
        Guide to "Guide",
        Start to "Start",
        LT to "LT",
        LB to "LB",
        RB to "RB",
        RT to "RT",
    )
}

/** O que uma peça é e como responde ao toque. */
@Serializable
enum class WidgetKind {
    /** Um botão que aciona a máscara [PadWidget.mask]. */
    Button,

    /** Analógico do lado [PadWidget.left]. */
    Stick,

    /** Direcional de 8 vias. */
    Dpad,

    /** ABXY, os quatro botões movidos juntos. */
    Face,

    /** Gatilho analógico do lado [PadWidget.left]: a altura do dedo é a pressão. */
    Trigger,
}

/** Forma e tamanho-base de um [WidgetKind.Button]. */
@Serializable
enum class ButtonStyle {
    /** Disco pequeno (Back, Start). */
    Round,

    /** Disco maior (Guide). */
    RoundLarge,

    /** Pílula larga (ombros). */
    Pill,
}

/**
 * Onde e como uma peça aparece. `x` e `y` são o **centro** em fração da largura e da altura
 * da tela (0–1), então o layout vale em qualquer tamanho de tela.
 */
@Serializable
data class ElementSpec(val x: Float, val y: Float, val scale: Float = 1f, val opacity: Float = 1f, val visible: Boolean = true)

/**
 * Como um analógico responde ao dedo, à parte do tamanho da peça.
 * - `floating`: o centro nasce onde o dedo pousa; falso = o centro é fixo no meio da zona.
 * - `travelScale`: multiplica o curso (a distância até a deflexão máxima).
 * - `deadzone`: fração do curso (0–[PadLayouts.MAX_DEADZONE]) em que o toque é ignorado; é
 *   independente da zona morta do perfil no PC.
 */
@Serializable
data class StickConfig(val floating: Boolean = true, val travelScale: Float = 1f, val deadzone: Float = 0f)

/**
 * Uma peça do layout. A ordem em [PadLayout.widgets] é a ordem de desenho.
 * Só os campos do [kind] valem: `left` para analógico e gatilho, `mask`/`style`/`label` para
 * botão e `stick` para analógico.
 */
@Serializable
data class PadWidget(
    val id: String,
    val kind: WidgetKind = WidgetKind.Button,
    val spec: ElementSpec,
    val left: Boolean = true,
    val mask: Int = 0,
    val style: ButtonStyle = ButtonStyle.Round,
    /** Texto desenhado no botão; vazio usa o padrão da máscara (LB, RB, A...). */
    val label: String = "",
    val stick: StickConfig = StickConfig(),
) {
    /** O texto desenhado na peça: o escolhido pelo usuário ou o padrão do tipo. */
    fun displayLabel(): String = label.ifEmpty {
        when (kind) {
            WidgetKind.Trigger -> if (left) "LT" else "RT"
            WidgetKind.Button -> defaultLabel(mask)
            else -> ""
        }
    }

    private fun defaultLabel(mask: Int) = when (mask) {
        PadButtons.LEFT_SHOULDER -> "LB"
        PadButtons.RIGHT_SHOULDER -> "RB"
        PadButtons.A -> "A"
        PadButtons.B -> "B"
        PadButtons.X -> "X"
        PadButtons.Y -> "Y"
        else -> ""
    }
}

/** Um layout salvo: o nome que o usuário vê e as peças, na ordem em que são desenhadas. */
@Serializable
data class PadLayout(val id: String, val name: String, val widgets: List<PadWidget> = PadLayouts.DefaultWidgets, val skin: PadSkin = PadSkin()) {
    fun widget(id: String): PadWidget? = widgets.firstOrNull { it.id == id }

    /** Onde a peça está; uma peça que o layout não tem cai no padrão (ou no centro, se nem isso). */
    fun spec(id: String): ElementSpec = widget(id)?.spec ?: PadLayouts.DefaultSpecs[id] ?: ElementSpec(0.5f, 0.5f)

    fun with(id: String, spec: ElementSpec): PadLayout = mapWidget(id) { it.copy(spec = spec) }

    fun mapWidget(id: String, change: (PadWidget) -> PadWidget): PadLayout =
        if (widgets.none { it.id == id }) this else copy(widgets = widgets.map { if (it.id == id) change(it) else it })

    /** Atalho para o analógico do [left] (o primeiro, se houver mais de um). */
    fun stick(left: Boolean): StickConfig = widgets.firstOrNull { it.kind == WidgetKind.Stick && it.left == left }?.stick ?: StickConfig()

    fun withStick(left: Boolean, config: StickConfig): PadLayout {
        val target = widgets.firstOrNull { it.kind == WidgetKind.Stick && it.left == left } ?: return this
        return mapWidget(target.id) { it.copy(stick = config) }
    }
}

/** Os modelos de partida oferecidos no editor. */
enum class PadPreset { Default, Compact, LeftHanded, DpadOnly }

object PadLayouts {
    const val DefaultId = "default"

    const val MIN_SCALE = 0.6f
    const val MAX_SCALE = 1.6f
    const val MIN_OPACITY = 0.2f
    const val MIN_TRAVEL = 0.6f
    const val MAX_TRAVEL = 1.6f
    const val MAX_DEADZONE = 0.4f

    /** Teto de peças num layout (um arquivo importado não pode lotar a tela). */
    const val MAX_WIDGETS = 40

    /** O desenho original do controle (T-905). */
    val DefaultWidgets: List<PadWidget> = listOf(
        PadWidget(PadIds.LT, WidgetKind.Trigger, ElementSpec(0.10f, 0.12f), left = true),
        PadWidget(PadIds.RT, WidgetKind.Trigger, ElementSpec(0.90f, 0.12f), left = false),
        PadWidget(PadIds.LB, WidgetKind.Button, ElementSpec(0.26f, 0.12f), mask = PadButtons.LEFT_SHOULDER, style = ButtonStyle.Pill),
        PadWidget(PadIds.RB, WidgetKind.Button, ElementSpec(0.74f, 0.12f), mask = PadButtons.RIGHT_SHOULDER, style = ButtonStyle.Pill),
        PadWidget(PadIds.Back, WidgetKind.Button, ElementSpec(0.43f, 0.30f), mask = PadButtons.BACK, style = ButtonStyle.Round),
        PadWidget(PadIds.Guide, WidgetKind.Button, ElementSpec(0.50f, 0.30f), mask = PadButtons.GUIDE, style = ButtonStyle.RoundLarge),
        PadWidget(PadIds.Start, WidgetKind.Button, ElementSpec(0.57f, 0.30f), mask = PadButtons.START, style = ButtonStyle.Round),
        PadWidget(PadIds.Dpad, WidgetKind.Dpad, ElementSpec(0.32f, 0.74f)),
        PadWidget(PadIds.Face, WidgetKind.Face, ElementSpec(0.86f, 0.46f)),
        PadWidget(PadIds.LeftStick, WidgetKind.Stick, ElementSpec(0.14f, 0.47f), left = true),
        PadWidget(PadIds.RightStick, WidgetKind.Stick, ElementSpec(0.68f, 0.75f), left = false),
    )

    /** Onde cada peça padrão nasce (e para onde "restaurar" a devolve). */
    val DefaultSpecs: Map<String, ElementSpec> = DefaultWidgets.associate { it.id to it.spec }

    fun default(name: String = "Padrão"): PadLayout = PadLayout(DefaultId, name, DefaultWidgets)

    /** Monta o layout de um modelo, mantendo `id` e `name` de quem o recebe. */
    fun fromPreset(preset: PadPreset, id: String, name: String): PadLayout {
        val specs: Map<String, ElementSpec> = when (preset) {
            PadPreset.Default -> DefaultSpecs

            PadPreset.Compact -> DefaultSpecs.mapValues { (element, s) ->
                when (element) {
                    // Encolhe tudo e aproxima dos cantos, para mãos pequenas ou telas menores.
                    PadIds.LeftStick -> s.copy(x = 0.12f, scale = 0.85f)

                    PadIds.RightStick -> s.copy(x = 0.70f, scale = 0.85f)

                    PadIds.Dpad -> s.copy(x = 0.28f, scale = 0.85f)

                    PadIds.Face -> s.copy(x = 0.88f, scale = 0.85f)

                    else -> s.copy(scale = 0.85f)
                }
            }

            // Espelho: o que era da esquerda vai para a direita, mantendo a função de cada peça.
            PadPreset.LeftHanded -> DefaultSpecs.mapValues { (_, s) -> s.copy(x = 1f - s.x) }

            // Retrô: sem analógicos nem gatilhos; o direcional cresce e os ombros ficam.
            PadPreset.DpadOnly -> DefaultSpecs.mapValues { (element, s) ->
                when (element) {
                    PadIds.LeftStick, PadIds.RightStick, PadIds.LT, PadIds.RT -> s.copy(visible = false)
                    PadIds.Dpad -> s.copy(x = 0.20f, y = 0.60f, scale = 1.3f)
                    PadIds.Face -> s.copy(x = 0.82f, y = 0.58f, scale = 1.1f)
                    PadIds.LB -> s.copy(x = 0.14f)
                    PadIds.RB -> s.copy(x = 0.86f)
                    else -> s
                }
            }
        }
        return PadLayout(id, name, DefaultWidgets.map { it.copy(spec = specs.getValue(it.id)) })
    }
}

/**
 * Lê um layout do formato antigo (v1: as 11 peças num mapa por nome, mais a configuração dos dois
 * analógicos) e o entrega já no formato de widgets. O que já está em widgets passa direto.
 * É usado onde um layout é lido de fora (configurações salvas e arquivos importados).
 */
object PadLayoutSerializer : JsonTransformingSerializer<PadLayout>(PadLayout.serializer()) {
    private val json = Json {
        ignoreUnknownKeys = true
        coerceInputValues = true
        encodeDefaults = true
    }

    @Serializable
    private class LegacyLayout(
        val id: String = "layout",
        val name: String = "Layout",
        val elements: Map<String, ElementSpec> = emptyMap(),
        val leftStick: StickConfig = StickConfig(),
        val rightStick: StickConfig = StickConfig(),
        val skin: PadSkin = PadSkin(),
    ) {
        fun upgrade() = PadLayout(
            id = id,
            name = name,
            widgets = PadLayouts.DefaultWidgets.map { w ->
                w.copy(
                    spec = elements[PadIds.LegacyNames[w.id]] ?: w.spec,
                    stick = when (w.id) {
                        PadIds.LeftStick -> leftStick
                        PadIds.RightStick -> rightStick
                        else -> w.stick
                    },
                )
            },
            skin = skin,
        )
    }

    override fun transformDeserialize(element: JsonElement): JsonElement {
        val obj = element as? JsonObject ?: return element
        if ("widgets" in obj) return obj
        val legacy = runCatching { json.decodeFromJsonElement(LegacyLayout.serializer(), obj) }.getOrNull() ?: return obj
        return json.encodeToJsonElement(PadLayout.serializer(), legacy.upgrade())
    }
}

/**
 * As operações do editor, sem Compose: mover com ímã na grade, redimensionar e mudar a opacidade
 * sempre dentro dos limites. Devolvem um layout novo (os dados são imutáveis).
 */
object PadEditing {
    /** Passo da grade do ímã, em fração da altura da tela (≈ 5 % da altura). */
    const val GRID = 0.05f

    /** Folga mínima entre o centro da peça e a borda da tela. */
    private const val EDGE = 0.03f

    /**
     * Move o centro da peça para `(x, y)` (frações), com ímã na grade se [snap] e preso à tela.
     * A grade usa a mesma distância em pixels nos dois eixos: o passo em `x` é `GRID * altura / largura`.
     */
    fun moveTo(layout: PadLayout, id: String, x: Float, y: Float, aspect: Float, snap: Boolean): PadLayout {
        val sx = if (snap) snapTo(x, GRID / aspect) else x
        val sy = if (snap) snapTo(y, GRID) else y
        return layout.with(id, layout.spec(id).copy(x = sx.coerceIn(EDGE, 1f - EDGE), y = sy.coerceIn(EDGE, 1f - EDGE)))
    }

    fun setScale(layout: PadLayout, id: String, scale: Float): PadLayout =
        layout.with(id, layout.spec(id).copy(scale = scale.coerceIn(PadLayouts.MIN_SCALE, PadLayouts.MAX_SCALE)))

    fun setOpacity(layout: PadLayout, id: String, opacity: Float): PadLayout =
        layout.with(id, layout.spec(id).copy(opacity = opacity.coerceIn(PadLayouts.MIN_OPACITY, 1f)))

    fun setVisible(layout: PadLayout, id: String, visible: Boolean): PadLayout = layout.with(id, layout.spec(id).copy(visible = visible))

    fun setStick(layout: PadLayout, id: String, config: StickConfig): PadLayout = layout.mapWidget(id) { it.copy(stick = clamp(config)) }

    /** Atalho para o analógico do [left]. */
    fun setStick(layout: PadLayout, left: Boolean, config: StickConfig): PadLayout = layout.withStick(left, clamp(config))

    private fun clamp(config: StickConfig) = config.copy(
        travelScale = config.travelScale.coerceIn(PadLayouts.MIN_TRAVEL, PadLayouts.MAX_TRAVEL),
        deadzone = config.deadzone.coerceIn(0f, PadLayouts.MAX_DEADZONE),
    )

    /** Devolve só esta peça ao lugar e tamanho originais (peças que não são do padrão ficam como estão). */
    fun resetElement(layout: PadLayout, id: String): PadLayout {
        val original = PadLayouts.DefaultWidgets.firstOrNull { it.id == id } ?: return layout
        return layout.mapWidget(id) { it.copy(spec = original.spec, stick = original.stick) }
    }

    private fun snapTo(value: Float, step: Float): Float = kotlin.math.round(value / step) * step
}

object StickMath {
    /**
     * Zona morta radial com reescala: abaixo de `dz` (fração do curso) a saída é 0, e de `dz` a 1
     * a saída vai de 0 a 1 mantendo a direção, sem salto na borda. `(vx, vy)` já está no círculo unitário.
     */
    fun deadzone(vx: Float, vy: Float, dz: Float): Pair<Float, Float> {
        if (dz <= 0f) return vx to vy
        val length = kotlin.math.sqrt(vx * vx + vy * vy)
        if (length <= dz) return 0f to 0f
        val k = ((length - dz) / (1f - dz)) / length
        return vx * k to vy * k
    }
}
