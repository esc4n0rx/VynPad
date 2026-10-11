package app.vynpad.ui.screens

import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.unit.dp
import app.vynpad.input.GamepadState
import app.vynpad.pairing.SessionState
import app.vynpad.resources.Res
import app.vynpad.resources.gamepad_layout
import app.vynpad.resources.gamepad_leave
import app.vynpad.resources.gamepad_menu_title
import app.vynpad.resources.gamepad_player_n
import app.vynpad.resources.gamepad_preview
import app.vynpad.resources.gamepad_resume
import app.vynpad.resources.gyro_hold_still
import app.vynpad.resources.gyro_recenter
import app.vynpad.resources.layout_edit
import app.vynpad.resources.session_disconnect
import app.vynpad.settings.AppSettings
import app.vynpad.skin.SkinBitmap
import app.vynpad.skin.rememberSkinStore
import app.vynpad.ui.PlatformBackHandler
import app.vynpad.ui.components.ButtonKind
import app.vynpad.ui.components.GlassCard
import app.vynpad.ui.components.VynButton
import app.vynpad.ui.components.VynChip
import app.vynpad.ui.components.VynIcon
import app.vynpad.ui.components.VynIconView
import app.vynpad.ui.components.VynSheet
import app.vynpad.ui.components.VynText
import app.vynpad.ui.components.formatRtt
import app.vynpad.ui.components.rttColor
import app.vynpad.ui.gamepad.GamepadSurface
import app.vynpad.ui.gamepad.PadEditing
import app.vynpad.ui.gamepad.PadEditorOverlay
import app.vynpad.ui.gamepad.PadFeedback
import app.vynpad.ui.gamepad.WidgetKind
import app.vynpad.ui.haptics.LocalHaptics
import app.vynpad.ui.theme.VynTheme
import kotlinx.coroutines.flow.StateFlow
import org.jetbrains.compose.resources.stringResource

/**
 * Tela do gamepad: paisagem, imersiva e com a tela sempre ligada (o chamador liga isso).
 * O controle ocupa tudo; só há uma pílula de status no topo (jogador, PC) que abre o menu.
 */
@Composable
fun GamepadScreen(
    state: GamepadState,
    session: SessionState,
    rttMicros: Int?,
    gyroActive: Boolean,
    gyroCalibrating: Boolean,
    onRecenterGyro: () -> Unit,
    tilt: StateFlow<Float?>,
    onWantTilt: (Boolean) -> Unit,
    settings: AppSettings,
    onSettings: (AppSettings) -> Unit,
    onBack: () -> Unit,
    onDisconnect: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    val haptics = LocalHaptics.current
    var menuOpen by remember { mutableStateOf(false) }

    // Editor de layout: `draft` é o layout em edição (a tela já o mostra enquanto se arrasta);
    // só vai para os ajustes (e para o disco) quando o dedo solta.
    var editing by remember { mutableStateOf(false) }
    var selected by remember { mutableStateOf<String?>(null) }
    var snap by remember { mutableStateOf(true) }
    var draft by remember(settings.activeLayout) { mutableStateOf(settings.activeLayout) }
    val layout = if (editing) draft else settings.activeLayout
    val skin = layout.skin

    // O fundo é lido do disco fora da tela principal e só troca quando a imagem ou o desfoque mudam.
    val skinStore = rememberSkinStore()
    val background by produceState<SkinBitmap?>(null, skin.image, skin.blur) {
        value = skin.image?.let { skinStore.load(it, skin.blur) }
    }
    var screenAspect by remember { mutableStateOf(16f / 9f) }

    val connected = session as? SessionState.Connected
    val slot = connected?.slot ?: 0
    val accent = when {
        skin.useImageAccent && skin.accent != null -> Color(skin.accent.toInt())
        settings.slotAccent -> c.slot(slot)
        else -> c.accent
    }

    // O acelerômetro só roda enquanto há um volante por inclinação à vista e o controle está em uso.
    val needsTilt = !editing && layout.widgets.any { it.kind == WidgetKind.Wheel && it.spec.visible && it.wheel.tilt }
    DisposableEffect(needsTilt) {
        onWantTilt(needsTilt)
        onDispose { onWantTilt(false) }
    }

    val feedback = remember(haptics) {
        object : PadFeedback {
            override fun press() = haptics.press()
            override fun slide() = haptics.tick()
            override fun detent() = haptics.tick()
        }
    }

    // Voltar abre o menu (em vez de sair sem querer no meio do jogo); com o menu aberto, fecha-o.
    PlatformBackHandler(enabled = true) {
        if (editing) {
            editing = false
            selected = null
        } else {
            menuOpen = !menuOpen
        }
    }

    val pulse by rememberInfiniteTransition().animateFloat(
        initialValue = 0.5f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(tween(1400), RepeatMode.Reverse),
    )

    Box(modifier.fillMaxSize().onSizeChanged { if (it.height > 0) screenAspect = it.width.toFloat() / it.height }) {
        GamepadSurface(
            state = state,
            accent = accent,
            feedback = feedback,
            layout = layout,
            background = background,
            tilt = tilt,
            enabled = !menuOpen && !editing,
        )

        if (editing) {
            PadEditorOverlay(
                layout = draft,
                selected = selected,
                snap = snap,
                onSelect = { selected = it },
                onMove = { element, x, y, aspect ->
                    draft = PadEditing.moveTo(draft, element, x, y, aspect, snap)
                },
                onMoveEnd = { onSettings(settings.withLayout(draft)) },
            )
            LayoutEditorChrome(
                settings = settings,
                draft = draft,
                selected = selected,
                snap = snap,
                background = background,
                aspect = screenAspect,
                onSnap = { snap = it },
                onSelect = { selected = it },
                onDraft = { draft = it },
                onCommit = { onSettings(settings.withLayout(it)) },
                onSettings = onSettings,
                onDone = {
                    editing = false
                    selected = null
                },
            )
        }

        // Pílula de status: jogador e PC; tocar abre o menu.
        if (!editing) {
            GlassCard(
                Modifier
                    .align(Alignment.TopCenter)
                    .padding(top = 10.dp)
                    .clip(CircleShape)
                    .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {
                        haptics.tick()
                        menuOpen = true
                    },
                radius = 28.dp,
                base = c.windowSolid.copy(alpha = 0.7f),
            ) {
                Row(
                    Modifier.padding(horizontal = 16.dp, vertical = 9.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Box(Modifier.size(10.dp).alpha(if (connected != null) pulse else 0.4f).background(accent, CircleShape))
                    VynText(
                        if (connected != null) {
                            stringResource(Res.string.gamepad_player_n, slot + 1) + " · " + connected.serverName
                        } else {
                            stringResource(Res.string.gamepad_preview)
                        },
                        style = t.bodyStrong,
                        maxLines = 1,
                    )
                    if (connected != null && rttMicros != null) {
                        VynText(formatRtt(rttMicros), style = t.caption, color = rttColor(rttMicros))
                    }
                    VynIconView(VynIcon.Menu, tint = c.textSecondary, size = 20.dp)
                }
            }
        }

        VynSheet(visible = menuOpen, onDismiss = { menuOpen = false }, maxWidth = 380.dp) {
            VynText(stringResource(Res.string.gamepad_menu_title), style = t.title)
            VynButton(stringResource(Res.string.gamepad_resume), { menuOpen = false }, modifier = Modifier.fillMaxWidth())
            if (settings.layouts.size > 1) {
                VynText(stringResource(Res.string.gamepad_layout), style = t.caption, color = c.textSecondary)
                Row(
                    Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    settings.layouts.forEach { l ->
                        VynChip(l.name, l.id == settings.activeLayoutId, { onSettings(settings.select(l.id)) })
                    }
                }
            }
            VynButton(
                stringResource(Res.string.layout_edit),
                {
                    menuOpen = false
                    selected = null
                    editing = true
                },
                kind = ButtonKind.Secondary,
                modifier = Modifier.fillMaxWidth(),
            )
            if (connected != null && gyroActive) {
                VynButton(
                    stringResource(if (gyroCalibrating) Res.string.gyro_hold_still else Res.string.gyro_recenter),
                    onRecenterGyro,
                    kind = ButtonKind.Secondary,
                    enabled = !gyroCalibrating,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
            if (connected != null) {
                VynButton(
                    stringResource(Res.string.session_disconnect),
                    {
                        menuOpen = false
                        onDisconnect()
                        onBack()
                    },
                    kind = ButtonKind.Destructive,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
            VynButton(
                stringResource(Res.string.gamepad_leave),
                {
                    menuOpen = false
                    onBack()
                },
                kind = ButtonKind.Secondary,
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}
