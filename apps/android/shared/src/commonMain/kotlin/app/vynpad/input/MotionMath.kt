package app.vynpad.input

import kotlin.math.abs
import kotlin.math.roundToInt

/** Vetor de três eixos (gyro em rad/s ou acelerômetro em m/s²). */
data class Vec3(val x: Float, val y: Float, val z: Float) {
    operator fun minus(o: Vec3) = Vec3(x - o.x, y - o.y, z - o.z)

    companion object {
        val Zero = Vec3(0f, 0f, 0f)
    }
}

/** Amostra pronta para o protocolo (PROTOCOLO §6.4): unidades inteiras e já saturadas. */
data class WireMotion(val gx: Int, val gy: Int, val gz: Int, val ax: Int, val ay: Int, val az: Int)

object MotionMath {
    /**
     * Gira os eixos do sensor (fixos no aparelho em retrato) para o referencial do controle:
     * X à direita, Y para cima, Z saindo da tela, com o aparelho na orientação em uso.
     * `quarterTurns` é a rotação da tela em múltiplos de 90° anti-horários (0, 1 = ROTATION_90, 2, 3).
     * Vale para vetores axiais (gyro) e polares (acelerômetro): a rotação é a mesma.
     */
    fun toControllerFrame(v: Vec3, quarterTurns: Int): Vec3 = when (((quarterTurns % 4) + 4) % 4) {
        1 -> Vec3(-v.y, v.x, v.z)
        2 -> Vec3(-v.x, -v.y, v.z)
        3 -> Vec3(v.y, -v.x, v.z)
        else -> v
    }

    /**
     * Inclinação do aparelho como um volante: o ângulo, em radianos, em que ele foi girado no
     * plano da tela (positivo = sentido horário, visto de frente), a partir da gravidade medida
     * pelo acelerômetro no referencial do controle. Em pé e reto vale 0.
     */
    fun roll(accel: Vec3): Float = kotlin.math.atan2(-accel.x, accel.y)

    /** `rad/s × 1000` e `m/s² × 100` em inteiros, saturando em 16 bits como manda o protocolo. */
    fun toWire(gyro: Vec3, accel: Vec3) = WireMotion(
        gx = sat(gyro.x * GYRO_SCALE),
        gy = sat(gyro.y * GYRO_SCALE),
        gz = sat(gyro.z * GYRO_SCALE),
        ax = sat(accel.x * ACCEL_SCALE),
        ay = sat(accel.y * ACCEL_SCALE),
        az = sat(accel.z * ACCEL_SCALE),
    )

    private fun sat(v: Float): Int = v.roundToInt().coerceIn(Short.MIN_VALUE.toInt(), Short.MAX_VALUE.toInt())

    private const val GYRO_SCALE = 1000f
    private const val ACCEL_SCALE = 100f
}

/**
 * Estima o viés (offset) do giroscópio com o aparelho parado.
 *
 * Junta amostras por [windowMillis]; se qualquer eixo oscilar mais que [stillnessLimit] em torno
 * da média (o aparelho mexeu), descarta a janela e recomeça. Ao fechar uma janela quieta, devolve
 * a média em [feed]. Os tempos vêm de fora (ms monotônicos), então é testável sem relógio.
 */
class GyroBiasEstimator(private val windowMillis: Long = 2_000, private val stillnessLimit: Float = 0.03f) {
    private var startedAt = -1L
    private var count = 0
    private var sx = 0.0
    private var sy = 0.0
    private var sz = 0.0
    private var minV = Vec3.Zero
    private var maxV = Vec3.Zero

    /** Progresso da janela atual, de 0 a 1. */
    fun progress(nowMillis: Long): Float = if (startedAt < 0) 0f else ((nowMillis - startedAt).toFloat() / windowMillis).coerceIn(0f, 1f)

    fun reset() {
        startedAt = -1
        count = 0
        sx = 0.0
        sy = 0.0
        sz = 0.0
    }

    /** Alimenta uma amostra bruta; devolve o viés quando uma janela quieta se completa. */
    fun feed(gyro: Vec3, nowMillis: Long): Vec3? {
        if (startedAt < 0) {
            startedAt = nowMillis
            minV = gyro
            maxV = gyro
        } else {
            minV = Vec3(minOf(minV.x, gyro.x), minOf(minV.y, gyro.y), minOf(minV.z, gyro.z))
            maxV = Vec3(maxOf(maxV.x, gyro.x), maxOf(maxV.y, gyro.y), maxOf(maxV.z, gyro.z))
        }
        // Mexeu: recomeça a janela a partir desta amostra.
        if (spread() > stillnessLimit) {
            reset()
            startedAt = nowMillis
            minV = gyro
            maxV = gyro
        }
        count++
        sx += gyro.x
        sy += gyro.y
        sz += gyro.z
        if (nowMillis - startedAt < windowMillis || count < MIN_SAMPLES) return null
        val bias = Vec3((sx / count).toFloat(), (sy / count).toFloat(), (sz / count).toFloat())
        reset()
        return bias
    }

    private fun spread() = maxOf(abs(maxV.x - minV.x), abs(maxV.y - minV.y), abs(maxV.z - minV.z))

    private companion object {
        const val MIN_SAMPLES = 50
    }
}
