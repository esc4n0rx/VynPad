package app.vynpad.ui.screens

import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.unit.dp
import app.vynpad.resources.Res
import app.vynpad.resources.gamepad_back
import app.vynpad.resources.gamepad_player
import app.vynpad.resources.gamepad_wip
import app.vynpad.ui.components.GlassCard
import app.vynpad.ui.components.VynIcon
import app.vynpad.ui.components.VynIconView
import app.vynpad.ui.components.VynText
import app.vynpad.ui.haptics.LocalHaptics
import app.vynpad.ui.theme.VynTheme
import org.jetbrains.compose.resources.stringResource

/**
 * Tela do gamepad: landscape, tela cheia e imersiva (o chamador aplica isso). Por ora é só o
 * invólucro (voltar, jogador e o glow do slot); o layout multitouch entra na subtask E.
 */
@Composable
fun GamepadScreen(onBack: () -> Unit, modifier: Modifier = Modifier) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    val haptics = LocalHaptics.current
    val slot = c.slot(0)
    val pulse by rememberInfiniteTransition().animateFloat(
        initialValue = 0.55f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(tween(1600), RepeatMode.Reverse),
    )
    Box(modifier.fillMaxSize()) {
        Row(
            Modifier.padding(20.dp).align(Alignment.TopStart),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Box(
                Modifier
                    .size(44.dp)
                    .background(c.control, CircleShape)
                    .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {
                        haptics.tick()
                        onBack()
                    },
                contentAlignment = Alignment.Center,
            ) {
                VynIconView(VynIcon.Back, tint = c.textPrimary, size = 22.dp)
            }
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Box(Modifier.size(10.dp).alpha(pulse).background(slot, CircleShape))
                VynText(stringResource(Res.string.gamepad_player), style = t.bodyStrong)
            }
        }
        GlassCard(Modifier.align(Alignment.Center), glow = slot) {
            Column(Modifier.padding(horizontal = 32.dp, vertical = 24.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                VynIconView(VynIcon.Gamepad, tint = slot, size = 40.dp)
                VynText(stringResource(Res.string.gamepad_wip), style = t.body, color = c.textSecondary, modifier = Modifier.padding(top = 10.dp))
            }
        }
    }
}
