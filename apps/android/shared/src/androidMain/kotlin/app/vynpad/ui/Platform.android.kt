package app.vynpad.ui

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.content.pm.ActivityInfo
import android.os.Build
import android.view.InputDevice
import androidx.activity.compose.BackHandler
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat

@Composable
actual fun PlatformBackHandler(enabled: Boolean, onBack: () -> Unit) {
    BackHandler(enabled = enabled, onBack = onBack)
}

private tailrec fun Context.findActivity(): Activity? = when (this) {
    is Activity -> this
    is ContextWrapper -> baseContext.findActivity()
    else -> null
}

@Composable
actual fun ImmersiveLandscapeEffect(active: Boolean) {
    val view = LocalView.current
    DisposableEffect(active, view) {
        val activity = view.context.findActivity()
        if (!active || activity == null) {
            onDispose { }
        } else {
            val previous = activity.requestedOrientation
            // Tela sempre ligada e toques entregues sem agrupar: latência mínima no controle.
            view.keepScreenOn = true
            if (Build.VERSION.SDK_INT >= 31) view.requestUnbufferedDispatch(InputDevice.SOURCE_TOUCHSCREEN)
            activity.requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
            val controller = WindowCompat.getInsetsController(activity.window, view)
            controller.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
            controller.hide(WindowInsetsCompat.Type.systemBars())
            onDispose {
                view.keepScreenOn = false
                controller.show(WindowInsetsCompat.Type.systemBars())
                activity.requestedOrientation = previous
            }
        }
    }
}
