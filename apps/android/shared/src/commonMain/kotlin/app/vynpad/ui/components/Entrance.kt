package app.vynpad.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.graphics.graphicsLayer
import app.vynpad.ui.theme.vynSpring
import kotlinx.coroutines.delay

/** Entrada escalonada: fade + subida com a mola padrão, `index` define o atraso (60 ms cada). */
fun Modifier.entrance(index: Int = 0, offsetPx: Float = 40f): Modifier = composed {
    val progress = remember { Animatable(0f) }
    LaunchedEffect(Unit) {
        delay(index * 60L)
        progress.animateTo(1f, vynSpring())
    }
    graphicsLayer {
        alpha = progress.value.coerceIn(0f, 1f)
        translationY = (1f - progress.value) * offsetPx
    }
}
