package app.vynpad.ui.gamepad

import kotlinx.serialization.Serializable

/** Cada peça do controle que o usuário pode mover, redimensionar, esconder ou deixar translúcida. */
@Serializable
enum class PadElement {
    LeftStick,
    RightStick,
    Dpad,

    /** Os quatro botões ABXY, movidos juntos. */
    Face,
    Back,
    Guide,
    Start,
    LT,
    LB,
    RB,
    RT,
}

/**
 * Onde e como uma peça aparece. `x` e `y` são o **centro** em fração da largura e da altura
 * da tela (0–1), então o layout vale em qualquer tamanho de tela.
 */
@Serializable
data class ElementSpec(
    val x: Float,
    val y: Float,
    val scale: Float = 1f,
    val opacity: Float = 1f,
    val visible: Boolean = true,
)

/** Um layout salvo: o nome que o usuário vê e a posição de cada peça. */
@Serializable
data class PadLayout(
    val id: String,
    val name: String,
    val elements: Map<PadElement, ElementSpec> = emptyMap(),
) {
    /** Peças que o layout não menciona caem no padrão (um layout antigo continua válido). */
    fun spec(element: PadElement): ElementSpec = elements[element] ?: PadLayouts.DefaultSpecs.getValue(element)

    fun with(element: PadElement, spec: ElementSpec): PadLayout = copy(elements = fullMap() + (element to spec))

    /** Todas as peças, preenchendo o que faltar com o padrão. */
    fun fullMap(): Map<PadElement, ElementSpec> = PadElement.entries.associateWith(::spec)
}

/** Os modelos de partida oferecidos no editor. */
enum class PadPreset { Default, Compact, LeftHanded, DpadOnly }

object PadLayouts {
    const val DefaultId = "default"

    const val MIN_SCALE = 0.6f
    const val MAX_SCALE = 1.6f
    const val MIN_OPACITY = 0.2f

    /** O desenho original do controle (T-905). */
    val DefaultSpecs: Map<PadElement, ElementSpec> = mapOf(
        PadElement.LeftStick to ElementSpec(0.14f, 0.47f),
        PadElement.RightStick to ElementSpec(0.68f, 0.75f),
        PadElement.Dpad to ElementSpec(0.32f, 0.74f),
        PadElement.Face to ElementSpec(0.86f, 0.46f),
        PadElement.Back to ElementSpec(0.43f, 0.30f),
        PadElement.Guide to ElementSpec(0.50f, 0.30f),
        PadElement.Start to ElementSpec(0.57f, 0.30f),
        PadElement.LT to ElementSpec(0.10f, 0.12f),
        PadElement.LB to ElementSpec(0.26f, 0.12f),
        PadElement.RB to ElementSpec(0.74f, 0.12f),
        PadElement.RT to ElementSpec(0.90f, 0.12f),
    )

    fun default(name: String = "Padrão"): PadLayout = PadLayout(DefaultId, name, DefaultSpecs)

    /** Monta o layout de um modelo, mantendo `id` e `name` de quem o recebe. */
    fun fromPreset(preset: PadPreset, id: String, name: String): PadLayout {
        val specs: Map<PadElement, ElementSpec> = when (preset) {
            PadPreset.Default -> DefaultSpecs
            PadPreset.Compact -> DefaultSpecs.mapValues { (element, s) ->
                when (element) {
                    // Encolhe tudo e aproxima dos cantos, para mãos pequenas ou telas menores.
                    PadElement.LeftStick -> s.copy(x = 0.12f, scale = 0.85f)
                    PadElement.RightStick -> s.copy(x = 0.70f, scale = 0.85f)
                    PadElement.Dpad -> s.copy(x = 0.28f, scale = 0.85f)
                    PadElement.Face -> s.copy(x = 0.88f, scale = 0.85f)
                    else -> s.copy(scale = 0.85f)
                }
            }
            // Espelho: o que era da esquerda vai para a direita, mantendo a função de cada peça.
            PadPreset.LeftHanded -> DefaultSpecs.mapValues { (_, s) -> s.copy(x = 1f - s.x) }
            // Retrô: sem analógicos nem gatilhos; o direcional cresce e os ombros ficam.
            PadPreset.DpadOnly -> DefaultSpecs.mapValues { (element, s) ->
                when (element) {
                    PadElement.LeftStick, PadElement.RightStick, PadElement.LT, PadElement.RT ->
                        s.copy(visible = false)
                    PadElement.Dpad -> s.copy(x = 0.20f, y = 0.60f, scale = 1.3f)
                    PadElement.Face -> s.copy(x = 0.82f, y = 0.58f, scale = 1.1f)
                    PadElement.LB -> s.copy(x = 0.14f)
                    PadElement.RB -> s.copy(x = 0.86f)
                    else -> s
                }
            }
        }
        return PadLayout(id, name, specs)
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
    fun moveTo(layout: PadLayout, element: PadElement, x: Float, y: Float, aspect: Float, snap: Boolean): PadLayout {
        val sx = if (snap) snapTo(x, GRID / aspect) else x
        val sy = if (snap) snapTo(y, GRID) else y
        return layout.with(
            element,
            layout.spec(element).copy(x = sx.coerceIn(EDGE, 1f - EDGE), y = sy.coerceIn(EDGE, 1f - EDGE)),
        )
    }

    fun setScale(layout: PadLayout, element: PadElement, scale: Float): PadLayout =
        layout.with(element, layout.spec(element).copy(scale = scale.coerceIn(PadLayouts.MIN_SCALE, PadLayouts.MAX_SCALE)))

    fun setOpacity(layout: PadLayout, element: PadElement, opacity: Float): PadLayout =
        layout.with(element, layout.spec(element).copy(opacity = opacity.coerceIn(PadLayouts.MIN_OPACITY, 1f)))

    fun setVisible(layout: PadLayout, element: PadElement, visible: Boolean): PadLayout =
        layout.with(element, layout.spec(element).copy(visible = visible))

    /** Devolve só esta peça ao lugar e tamanho originais. */
    fun resetElement(layout: PadLayout, element: PadElement): PadLayout =
        layout.with(element, PadLayouts.DefaultSpecs.getValue(element))

    private fun snapTo(value: Float, step: Float): Float = kotlin.math.round(value / step) * step
}
