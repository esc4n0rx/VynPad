package app.vynpad.ui.haptics

import android.content.Context
import android.os.Build
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import android.view.HapticFeedbackConstants
import android.view.View
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalView

private class AndroidHaptics(private val view: View, private val strength: HapticStrength) : Haptics {
    private val vibrator: Vibrator? = if (Build.VERSION.SDK_INT >= 31) {
        (view.context.getSystemService(Context.VIBRATOR_MANAGER_SERVICE) as? VibratorManager)?.defaultVibrator
    } else {
        @Suppress("DEPRECATION")
        view.context.getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator
    }
    private val precise = vibrator?.hasAmplitudeControl() == true

    // Com controle de amplitude o toque tem a força escolhida; sem ele, vale o padrão do sistema.
    override fun press() = if (precise) pulse(PRESS_MILLIS, strength.amplitude) else perform(HapticFeedbackConstants.KEYBOARD_TAP)

    override fun tick() = if (precise) {
        pulse(TICK_MILLIS, (strength.amplitude * TICK_RATIO).toInt())
    } else {
        perform(
            if (Build.VERSION.SDK_INT >= 23) HapticFeedbackConstants.CLOCK_TICK else HapticFeedbackConstants.KEYBOARD_TAP,
        )
    }

    private fun pulse(millis: Long, amplitude: Int) {
        runCatching { vibrator?.vibrate(VibrationEffect.createOneShot(millis, amplitude.coerceIn(1, 255))) }
    }

    override fun confirm() = perform(
        if (Build.VERSION.SDK_INT >= 30) HapticFeedbackConstants.CONFIRM else HapticFeedbackConstants.LONG_PRESS,
    )

    override fun reject() = perform(
        if (Build.VERSION.SDK_INT >= 30) HapticFeedbackConstants.REJECT else HapticFeedbackConstants.LONG_PRESS,
    )

    private fun perform(constant: Int) {
        view.performHapticFeedback(constant)
    }

    private companion object {
        const val PRESS_MILLIS = 14L
        const val TICK_MILLIS = 8L
        const val TICK_RATIO = 0.7f
    }
}

@Composable
actual fun rememberPlatformHaptics(enabled: Boolean, strength: HapticStrength): Haptics {
    val view = LocalView.current
    return remember(view, enabled, strength) { if (enabled) AndroidHaptics(view, strength) else NoHaptics }
}
