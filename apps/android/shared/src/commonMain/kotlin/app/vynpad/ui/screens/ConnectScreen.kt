package app.vynpad.ui.screens

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import app.vynpad.resources.Res
import app.vynpad.resources.app_name
import app.vynpad.resources.connect_empty_body
import app.vynpad.resources.connect_empty_title
import app.vynpad.resources.connect_preview_gamepad
import app.vynpad.resources.connect_search
import app.vynpad.resources.connect_searching
import app.vynpad.resources.connect_tagline
import app.vynpad.resources.status_offline
import app.vynpad.resources.status_searching
import app.vynpad.ui.components.BadgeTone
import app.vynpad.ui.components.ButtonKind
import app.vynpad.ui.components.GlassCard
import app.vynpad.ui.components.VynBadge
import app.vynpad.ui.components.VynButton
import app.vynpad.ui.components.VynIcon
import app.vynpad.ui.components.VynIconView
import app.vynpad.ui.components.VynText
import app.vynpad.ui.components.entrance
import app.vynpad.ui.haptics.LocalHaptics
import app.vynpad.ui.theme.VynTheme
import org.jetbrains.compose.resources.stringResource

@Composable
fun ConnectScreen(
    searching: Boolean,
    onToggleSearch: () -> Unit,
    onPreviewGamepad: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    val haptics = LocalHaptics.current
    Column(
        modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp),
    ) {
        Spacer(Modifier.height(16.dp))
        Column(Modifier.entrance(0), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            VynText(stringResource(Res.string.app_name), style = t.hero)
            VynText(stringResource(Res.string.connect_tagline), style = t.body, color = c.textSecondary)
        }

        GlassCard(Modifier.fillMaxWidth().entrance(1)) {
            Column(
                Modifier.fillMaxWidth().padding(24.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(14.dp),
            ) {
                Radar(searching)
                VynBadge(
                    text = stringResource(if (searching) Res.string.status_searching else Res.string.status_offline),
                    tone = if (searching) BadgeTone.Accent else BadgeTone.Neutral,
                )
                VynText(stringResource(Res.string.connect_empty_title), style = t.title, color = c.textPrimary)
                VynText(
                    stringResource(Res.string.connect_empty_body),
                    style = t.body.copy(textAlign = TextAlign.Center),
                    color = c.textSecondary,
                )
                Spacer(Modifier.height(4.dp))
                VynButton(
                    text = stringResource(if (searching) Res.string.connect_searching else Res.string.connect_search),
                    onClick = {
                        haptics.confirm()
                        onToggleSearch()
                    },
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }

        VynButton(
            text = stringResource(Res.string.connect_preview_gamepad),
            onClick = onPreviewGamepad,
            kind = ButtonKind.Secondary,
            modifier = Modifier.fillMaxWidth().entrance(2),
        )
    }
}

/** Radar: celular no centro e anéis de accent que expandem enquanto procura. */
@Composable
private fun Radar(active: Boolean) {
    val c = VynTheme.colors
    val transition = rememberInfiniteTransition()
    val phase by transition.animateFloat(
        initialValue = 0f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(tween(2400, easing = LinearEasing), RepeatMode.Restart),
    )
    Box(Modifier.size(150.dp), contentAlignment = Alignment.Center) {
        Canvas(Modifier.size(150.dp)) {
            val center = Offset(size.width / 2f, size.height / 2f)
            val max = size.minDimension / 2f
            drawCircle(c.accentSoft, radius = max * 0.52f, center = center)
            if (active) {
                listOf(0f, 0.5f).forEach { shift ->
                    val p = (phase + shift) % 1f
                    drawCircle(
                        color = c.accent.copy(alpha = (1f - p) * 0.55f),
                        radius = max * (0.5f + 0.5f * p),
                        center = center,
                        style = Stroke(width = 2.dp.toPx()),
                    )
                }
            } else {
                drawCircle(c.accent.copy(alpha = 0.2f), radius = max * 0.75f, center = center, style = Stroke(width = 1.dp.toPx()))
            }
        }
        Box(
            Modifier
                .size(64.dp)
                .background(Brush.verticalGradient(listOf(c.accent, c.indigo)), CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            VynIconView(VynIcon.Phone, tint = c.onAccent, size = 30.dp)
        }
    }
}
