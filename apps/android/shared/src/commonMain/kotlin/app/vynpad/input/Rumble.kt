package app.vynpad.input

/** Força da vibração do jogo no celular. */
enum class RumbleStrength(val scale: Float) {
    Soft(0.5f),
    Medium(0.75f),
    Strong(1f),
}

/** Ajustes da vibração que vem do jogo (rumble). */
data class RumbleSettings(
    val enabled: Boolean = true,
    val strength: RumbleStrength = RumbleStrength.Strong,
)

object RumbleMix {
    /** Peso do motor pequeno (alta frequência) ao combiná-lo com o grande numa só amplitude. */
    private const val SMALL_WEIGHT = 0.6f

    /**
     * Combina os dois motores do XInput (0–255) em uma amplitude de vibrador (0–255).
     * `0` só sai quando o jogo mandou parar (ou a força zera); qualquer rumble ativo vira ao menos `1`.
     */
    fun amplitude(large: Int, small: Int, strength: RumbleStrength): Int {
        val raw = maxOf(large.coerceIn(0, 255).toFloat(), small.coerceIn(0, 255) * SMALL_WEIGHT)
        if (raw <= 0f) return 0
        return (raw * strength.scale).toInt().coerceIn(1, 255)
    }
}
