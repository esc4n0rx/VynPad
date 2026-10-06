package app.vynpad.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.clickable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import app.vynpad.ui.haptics.LocalHaptics
import app.vynpad.ui.theme.VynRadius
import app.vynpad.ui.theme.VynTheme
import app.vynpad.ui.theme.vynSpring

enum class ButtonKind { Primary, Secondary, Ghost, Destructive }

/** Botão do design system; ao pressionar encolhe com a mola padrão e dá um toque de haptic. */
@Composable
fun VynButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    kind: ButtonKind = ButtonKind.Primary,
    enabled: Boolean = true,
) {
    val c = VynTheme.colors
    val haptics = LocalHaptics.current
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val scale by animateFloatAsState(if (pressed) 0.96f else 1f, vynSpring(0.001f))
    val container = when (kind) {
        ButtonKind.Primary -> c.accent
        ButtonKind.Secondary -> c.control
        ButtonKind.Ghost -> Color.Transparent
        ButtonKind.Destructive -> c.danger.copy(alpha = 0.16f)
    }
    val content = when (kind) {
        ButtonKind.Primary -> c.onAccent
        ButtonKind.Destructive -> c.danger
        else -> c.textPrimary
    }
    val shape = RoundedCornerShape(VynRadius.lg)
    Box(
        modifier
            .scale(scale)
            .graphicsLayer { alpha = if (enabled) 1f else 0.4f }
            .then(
                if (kind == ButtonKind.Primary) {
                    Modifier.shadow(14.dp, shape, ambientColor = c.accent, spotColor = c.accent)
                } else {
                    Modifier
                },
            )
            .clip(shape)
            .background(container)
            .then(
                if (kind == ButtonKind.Primary) {
                    Modifier.background(Brush.verticalGradient(listOf(Color.White.copy(alpha = 0.18f), Color.Transparent)))
                } else {
                    Modifier
                },
            )
            .clickable(interactionSource = interaction, indication = null, enabled = enabled) {
                haptics.press()
                onClick()
            }
            .height(50.dp)
            .padding(horizontal = 22.dp),
        contentAlignment = Alignment.Center,
    ) {
        VynText(text, style = VynTheme.typography.bodyStrong.copy(color = content, textAlign = TextAlign.Center))
    }
}

/** Toggle estilo macOS/iOS: a bolinha desliza com a mola padrão e o trilho muda de cor. */
@Composable
fun VynToggle(checked: Boolean, onCheckedChange: (Boolean) -> Unit, modifier: Modifier = Modifier) {
    val c = VynTheme.colors
    val haptics = LocalHaptics.current
    val track by animateColorAsState(if (checked) c.success else c.controlHover, vynSpring())
    val knobX by animateDpAsState(if (checked) 20.dp else 0.dp, vynSpring())
    Box(
        modifier
            .width(51.dp)
            .height(31.dp)
            .clip(CircleShape)
            .background(track)
            .pointerInput(checked) {
                detectTapGestures {
                    haptics.tick()
                    onCheckedChange(!checked)
                }
            }
            .padding(2.dp),
    ) {
        Box(
            Modifier
                .offset(x = knobX)
                .size(27.dp)
                .shadow(4.dp, CircleShape)
                .background(Color.White, CircleShape),
        )
    }
}

/** Controle segmentado: a pílula de seleção desliza entre as opções com a mola padrão. */
@Composable
fun <T> VynSegmented(
    options: List<Pair<T, String>>,
    selected: T,
    onSelect: (T) -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = VynTheme.colors
    val haptics = LocalHaptics.current
    val index = options.indexOfFirst { it.first == selected }.coerceAtLeast(0)
    BoxWithConstraints(
        modifier
            .fillMaxWidth()
            .height(40.dp)
            .clip(RoundedCornerShape(VynRadius.lg))
            .background(c.control)
            .padding(3.dp),
    ) {
        val segment = maxWidth / options.size
        val x by animateDpAsState(segment * index, vynSpring())
        Box(
            Modifier
                .offset(x = x)
                .width(segment)
                .height(34.dp)
                .shadow(3.dp, RoundedCornerShape(VynRadius.md))
                .background(if (c.isDark) Color(0xFF636366) else Color.White, RoundedCornerShape(VynRadius.md)),
        )
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.Start) {
            options.forEach { (value, label) ->
                val active = value == selected
                val color by animateColorAsState(if (active) c.textPrimary else c.textSecondary, vynSpring())
                Box(
                    Modifier
                        .width(segment)
                        .height(34.dp)
                        .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {
                            if (!active) {
                                haptics.tick()
                                onSelect(value)
                            }
                        },
                    contentAlignment = Alignment.Center,
                ) {
                    VynText(label, style = VynTheme.typography.bodyStrong.copy(color = color))
                }
            }
        }
    }
}

enum class BadgeTone { Neutral, Success, Warning, Danger, Info, Accent }

@Composable
fun VynBadge(text: String, tone: BadgeTone = BadgeTone.Neutral, modifier: Modifier = Modifier) {
    val c = VynTheme.colors
    val color = when (tone) {
        BadgeTone.Neutral -> c.textSecondary
        BadgeTone.Success -> c.success
        BadgeTone.Warning -> c.warning
        BadgeTone.Danger -> c.danger
        BadgeTone.Info -> c.info
        BadgeTone.Accent -> c.accent
    }
    Row(
        modifier
            .clip(CircleShape)
            .background(color.copy(alpha = 0.16f))
            .padding(horizontal = 10.dp, vertical = 5.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Box(Modifier.size(6.dp).background(color, CircleShape))
        VynText(text, style = VynTheme.typography.caption.copy(color = color))
    }
}
