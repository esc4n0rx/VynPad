package app.vynpad.ui.gamepad

import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow
import kotlinx.serialization.Serializable

/**
 * O visual de um layout: o banner de um jogo ao fundo, um véu escuro para os botões
 * continuarem legíveis, a cor de destaque tirada da imagem e a opacidade dos botões.
 * A imagem fica só no aparelho (`image` é o nome do arquivo no armazenamento do app).
 */
@Serializable
data class PadSkin(
    val image: String? = null,
    /** Escurecimento sobre a imagem, de 0 a [PadSkins.MAX_VEIL]. */
    val veil: Float = PadSkins.DEFAULT_VEIL,
    /** Usa a versão desfocada da imagem (calculada uma vez, na importação). */
    val blur: Boolean = false,
    /** Cor dominante da imagem, ARGB; nula se a imagem não tem uma cor marcante. */
    val accent: Long? = null,
    /** `true`: o controle usa [accent] no lugar da cor do jogador. */
    val useImageAccent: Boolean = true,
    /** Multiplica a opacidade de todas as peças. */
    val padOpacity: Float = 1f,
) {
    val hasImage: Boolean get() = image != null
}

object PadSkins {
    const val DEFAULT_VEIL = 0.5f
    const val MAX_VEIL = 0.85f
    const val MIN_PAD_OPACITY = 0.3f

    /**
     * Razão mínima de luminância entre o corpo dos botões e o fundo (já com o véu). É uma
     * heurística: o desenho padrão (grafite sobre o fundo escuro do app) fica em ~1,8.
     */
    const val MIN_CONTRAST = 1.6f
}

/**
 * Luminância relativa média da imagem, numa grade grosseira (`cols` × `rows`), para medir o
 * contraste sob cada peça sem varrer a imagem inteira a cada ajuste.
 */
class LumaGrid(val cols: Int, val rows: Int, private val values: FloatArray) {
    init {
        require(values.size == cols * rows) { "grade ${cols}x$rows precisa de ${cols * rows} valores" }
    }

    /** Média da região (frações da tela de 0 a 1); região fora da tela é recortada. */
    fun mean(left: Float, top: Float, right: Float, bottom: Float): Float {
        val c0 = (left.coerceIn(0f, 1f) * cols).toInt().coerceIn(0, cols - 1)
        val c1 = (right.coerceIn(0f, 1f) * cols).toInt().coerceIn(c0, cols - 1)
        val r0 = (top.coerceIn(0f, 1f) * rows).toInt().coerceIn(0, rows - 1)
        val r1 = (bottom.coerceIn(0f, 1f) * rows).toInt().coerceIn(r0, rows - 1)
        var sum = 0f
        var n = 0
        for (r in r0..r1) {
            for (c in c0..c1) {
                sum += values[r * cols + c]
                n++
            }
        }
        return sum / n
    }

    companion object {
        const val COLS = 48
        const val ROWS = 27

        /**
         * Reduz `pixels` (ARGB, `width` × `height`) à grade, **já cobrindo a tela inteira como a
         * imagem é desenhada** (corte central): quem chama passa a imagem na proporção da tela.
         */
        fun from(pixels: IntArray, width: Int, height: Int, cols: Int = COLS, rows: Int = ROWS): LumaGrid {
            // Imagem menor que a grade: a grade encolhe, para nenhuma célula ficar sem amostra.
            val cols = min(cols, width)
            val rows = min(rows, height)
            val sums = FloatArray(cols * rows)
            val counts = IntArray(cols * rows)
            for (y in 0 until height) {
                val r = (y * rows / height).coerceAtMost(rows - 1)
                for (x in 0 until width) {
                    val c = (x * cols / width).coerceAtMost(cols - 1)
                    sums[r * cols + c] += luminance(pixels[y * width + x])
                    counts[r * cols + c]++
                }
            }
            for (i in sums.indices) if (counts[i] > 0) sums[i] /= counts[i]
            return LumaGrid(cols, rows, sums)
        }
    }
}

/** Luminância relativa (WCAG, 0–1) de um pixel ARGB. */
fun luminance(argb: Int): Float {
    fun lin(v: Int): Float {
        val s = v / 255f
        return if (s <= 0.03928f) s / 12.92f else ((s + 0.055f) / 1.055f).pow(2.4f)
    }
    return 0.2126f * lin((argb shr 16) and 0xFF) + 0.7152f * lin((argb shr 8) and 0xFF) + 0.0722f * lin(argb and 0xFF)
}

/** Mede e corrige a legibilidade dos botões sobre a imagem. */
object SkinContrast {
    /** Razão de contraste (≥ 1) entre duas luminâncias. */
    fun ratio(a: Float, b: Float): Float = (max(a, b) + 0.05f) / (min(a, b) + 0.05f)

    /**
     * Pior contraste entre o corpo das peças visíveis (`fillLuma`) e a imagem sob elas depois do
     * véu; `null` se não há peça visível. Devolve a peça e a razão.
     */
    fun worst(luma: LumaGrid, g: GamepadGeometry, veil: Float, fillLuma: Float): Pair<String, Float>? {
        var worst: Pair<String, Float>? = null
        for (part in g.parts) {
            if (!part.visible) continue
            val b = part.bounds
            val bg = luma.mean(b.left / g.width, b.top / g.height, b.right / g.width, b.bottom / g.height) * (1f - veil)
            val r = ratio(fillLuma, bg)
            if (worst == null || r < worst.second) worst = part.id to r
        }
        return worst
    }

    fun isLegible(luma: LumaGrid, g: GamepadGeometry, veil: Float, fillLuma: Float): Boolean =
        (worst(luma, g, veil, fillLuma)?.second ?: Float.MAX_VALUE) >= PadSkins.MIN_CONTRAST

    /** O menor véu (em passos de 5 %), a partir do atual, que deixa tudo legível; no máximo [PadSkins.MAX_VEIL]. */
    fun autoVeil(luma: LumaGrid, g: GamepadGeometry, current: Float, fillLuma: Float): Float {
        var veil = current
        while (veil < PadSkins.MAX_VEIL) {
            if (isLegible(luma, g, veil, fillLuma)) return veil
            veil = min(veil + 0.05f, PadSkins.MAX_VEIL)
        }
        return PadSkins.MAX_VEIL
    }
}

/** Escolhe a cor de destaque de uma imagem. */
object AccentPicker {
    private const val HUE_BUCKETS = 24
    private const val MIN_SATURATION = 0.28f
    private const val MIN_VALUE = 0.25f

    /**
     * A cor mais marcante dos `pixels` (ARGB): conta os tons pesando por saturação e brilho, fica
     * com o tom dominante e o normaliza para servir de destaque sobre vidro escuro. Devolve `null`
     * quando a imagem é praticamente cinza (nenhuma cor marcante).
     */
    fun pick(pixels: IntArray): Long? {
        val weight = FloatArray(HUE_BUCKETS)
        val sumH = FloatArray(HUE_BUCKETS)
        val sumS = FloatArray(HUE_BUCKETS)
        val sumV = FloatArray(HUE_BUCKETS)
        for (p in pixels) {
            if ((p ushr 24) < 128) continue
            val (h, s, v) = hsv(p)
            if (s < MIN_SATURATION || v < MIN_VALUE) continue
            val w = s * v
            val bucket = ((h / 360f) * HUE_BUCKETS).toInt().coerceIn(0, HUE_BUCKETS - 1)
            weight[bucket] += w
            sumH[bucket] += h * w
            sumS[bucket] += s * w
            sumV[bucket] += v * w
        }
        val best = weight.indices.maxByOrNull { weight[it] } ?: return null
        if (weight[best] <= 0f) return null
        val h = sumH[best] / weight[best]
        val s = (sumS[best] / weight[best]).coerceIn(0.55f, 1f)
        val v = (sumV[best] / weight[best]).coerceIn(0.7f, 0.95f)
        return 0xFF000000L or (argbFromHsv(h, s, v).toLong() and 0xFFFFFFL)
    }

    private fun hsv(argb: Int): Triple<Float, Float, Float> {
        val r = ((argb shr 16) and 0xFF) / 255f
        val g = ((argb shr 8) and 0xFF) / 255f
        val b = (argb and 0xFF) / 255f
        val mx = max(r, max(g, b))
        val mn = min(r, min(g, b))
        val d = mx - mn
        val h = when {
            d == 0f -> 0f
            mx == r -> 60f * (((g - b) / d) % 6f)
            mx == g -> 60f * ((b - r) / d + 2f)
            else -> 60f * ((r - g) / d + 4f)
        }.let { if (it < 0f) it + 360f else it }
        return Triple(h, if (mx == 0f) 0f else d / mx, mx)
    }

    private fun argbFromHsv(h: Float, s: Float, v: Float): Int {
        val c = v * s
        val x = c * (1f - kotlin.math.abs((h / 60f) % 2f - 1f))
        val m = v - c
        val (r, g, b) = when ((h / 60f).toInt() % 6) {
            0 -> Triple(c, x, 0f)
            1 -> Triple(x, c, 0f)
            2 -> Triple(0f, c, x)
            3 -> Triple(0f, x, c)
            4 -> Triple(x, 0f, c)
            else -> Triple(c, 0f, x)
        }
        fun ch(f: Float) = ((f + m) * 255f + 0.5f).toInt().coerceIn(0, 255)
        return (0xFF shl 24) or (ch(r) shl 16) or (ch(g) shl 8) or ch(b)
    }
}
