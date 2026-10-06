package app.vynpad.pairing

import android.content.Context
import android.os.Build
import android.os.SystemClock
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import app.vynpad.input.RumbleMix
import app.vynpad.input.RumbleSettings
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

/** A cada quanto o efeito é renovado enquanto o jogo mantém o rumble ligado. */
private const val RENEW_MILLIS = 100L

/** Duração de cada efeito: um pouco além da renovação, para não haver vão entre eles. */
private const val EFFECT_MILLIS = 150L

/** Sem nenhum pacote do PC por este tempo, a vibração para (PROTOCOLO §9). */
private const val SILENCE_LIMIT_MILLIS = 2_000L

private const val TEST_MILLIS = 350L

/**
 * Transforma o rumble do jogo em vibração do celular.
 *
 * [onRumble] e [onAlive] podem ser chamados de qualquer thread sem bloquear: só gravam o estado
 * desejado; quem fala com o vibrador (Binder) é uma coroutine própria. Ao chegar `0` o vibrador é
 * cancelado na hora, sem esperar o fim do efeito em curso.
 */
@OptIn(ExperimentalCoroutinesApi::class)
internal class RumbleVibrator(context: Context, scope: CoroutineScope) {
    private val vibrator: Vibrator? = if (Build.VERSION.SDK_INT >= 31) {
        (context.getSystemService(Context.VIBRATOR_MANAGER_SERVICE) as? VibratorManager)?.defaultVibrator
    } else {
        @Suppress("DEPRECATION")
        context.getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator
    }

    @Volatile
    private var settings = RumbleSettings()

    private var large = 0
    private var small = 0

    /** Amplitude pedida (0 = parado). Mudar este valor reinicia o laço de renovação. */
    private val amplitude = MutableStateFlow(0)
    private val lastPacketAt = AtomicLong(SystemClock.elapsedRealtime())

    init {
        scope.launch {
            amplitude.collectLatest { level ->
                if (level <= 0) {
                    vibrator?.cancel()
                    return@collectLatest
                }
                while (isActive) {
                    if (SystemClock.elapsedRealtime() - lastPacketAt.get() > SILENCE_LIMIT_MILLIS) {
                        amplitude.compareAndSet(level, 0)
                        return@collectLatest
                    }
                    vibrate(level, EFFECT_MILLIS)
                    delay(RENEW_MILLIS)
                }
            }
        }
    }

    fun configure(next: RumbleSettings) {
        settings = next
        recompute()
    }

    /** O jogo mandou um estado novo dos motores. */
    fun onRumble(largeMotor: Int, smallMotor: Int) {
        lastPacketAt.set(SystemClock.elapsedRealtime())
        synchronized(this) {
            large = largeMotor
            small = smallMotor
        }
        recompute()
    }

    /** Chegou qualquer pacote válido do PC (pong, rumble): o canal está vivo. */
    fun onAlive() {
        lastPacketAt.set(SystemClock.elapsedRealtime())
    }

    /** A sessão acabou: nada de vibração sobrando. */
    fun reset() {
        synchronized(this) {
            large = 0
            small = 0
        }
        amplitude.value = 0
    }

    /** Vibra uma vez com a força escolhida, para o usuário calibrar. */
    fun test(testSettings: RumbleSettings) {
        val level = RumbleMix.amplitude(MAX_MOTOR, MAX_MOTOR, testSettings.strength)
        vibrate(level, TEST_MILLIS)
    }

    private fun recompute() {
        val s = settings
        val (l, sm) = synchronized(this) { large to small }
        amplitude.value = if (s.enabled) RumbleMix.amplitude(l, sm, s.strength) else 0
    }

    private fun vibrate(level: Int, millis: Long) {
        val v = vibrator ?: return
        if (!v.hasVibrator()) return
        val effect = if (v.hasAmplitudeControl()) {
            VibrationEffect.createOneShot(millis, level.coerceIn(1, 255))
        } else {
            VibrationEffect.createOneShot(millis, VibrationEffect.DEFAULT_AMPLITUDE)
        }
        runCatching { v.vibrate(effect) }
    }

    private companion object {
        const val MAX_MOTOR = 255
    }
}
