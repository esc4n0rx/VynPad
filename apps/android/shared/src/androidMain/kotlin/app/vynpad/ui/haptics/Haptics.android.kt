package app.vynpad.ui.haptics

import android.os.Build
import android.view.HapticFeedbackConstants
import android.view.View
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalView

private class AndroidHaptics(private val view: View) : Haptics {
    override fun press() = perform(HapticFeedbackConstants.KEYBOARD_TAP)

    override fun tick() = perform(
        if (Build.VERSION.SDK_INT >= 23) HapticFeedbackConstants.CLOCK_TICK else HapticFeedbackConstants.KEYBOARD_TAP,
    )

    override fun confirm() = perform(
        if (Build.VERSION.SDK_INT >= 30) HapticFeedbackConstants.CONFIRM else HapticFeedbackConstants.LONG_PRESS,
    )

    override fun reject() = perform(
        if (Build.VERSION.SDK_INT >= 30) HapticFeedbackConstants.REJECT else HapticFeedbackConstants.LONG_PRESS,
    )

    private fun perform(constant: Int) {
        view.performHapticFeedback(constant)
    }
}

@Composable
actual fun rememberPlatformHaptics(enabled: Boolean): Haptics {
    val view = LocalView.current
    return remember(view, enabled) { if (enabled) AndroidHaptics(view) else NoHaptics }
}
