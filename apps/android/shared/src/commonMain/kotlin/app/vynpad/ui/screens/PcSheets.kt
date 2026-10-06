package app.vynpad.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import app.vynpad.discovery.DiscoveredPc
import app.vynpad.discovery.DiscoveryVia
import app.vynpad.discovery.parseHostPort
import app.vynpad.resources.Res
import app.vynpad.resources.action_close
import app.vynpad.resources.manual_add
import app.vynpad.resources.manual_error
import app.vynpad.resources.manual_hint
import app.vynpad.resources.manual_title
import app.vynpad.resources.pc_address
import app.vynpad.resources.pc_fingerprint
import app.vynpad.resources.pc_forget
import app.vynpad.resources.pc_found_via
import app.vynpad.resources.pc_pair
import app.vynpad.resources.pc_slots
import app.vynpad.resources.pc_system
import app.vynpad.resources.session_connect
import app.vynpad.resources.session_disconnect
import app.vynpad.resources.via_broadcast
import app.vynpad.resources.via_manual
import app.vynpad.resources.via_mdns
import app.vynpad.ui.components.BadgeTone
import app.vynpad.ui.components.ButtonKind
import app.vynpad.ui.components.VynBadge
import app.vynpad.ui.components.VynButton
import app.vynpad.ui.components.VynIcon
import app.vynpad.ui.components.VynIconView
import app.vynpad.ui.components.VynSheet
import app.vynpad.ui.components.VynText
import app.vynpad.ui.haptics.LocalHaptics
import app.vynpad.ui.theme.VynRadius
import app.vynpad.ui.theme.VynTheme
import org.jetbrains.compose.resources.stringResource

/** Detalhes do PC tocado no radar ou na lista. O pareamento entra na subtask D. */
@Composable
fun PcSheet(
    pc: DiscoveredPc?,
    paired: Boolean,
    connected: Boolean,
    onPair: (DiscoveredPc) -> Unit,
    onConnect: (DiscoveredPc) -> Unit,
    onDisconnect: () -> Unit,
    onForget: (DiscoveredPc) -> Unit,
    onDismiss: () -> Unit,
) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    // Mantém o último PC para o conteúdo não sumir durante a animação de saída.
    var shown by remember { mutableStateOf(pc) }
    if (pc != null) shown = pc
    VynSheet(visible = pc != null, onDismiss = onDismiss) {
        val item = shown ?: return@VynSheet
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
            Box(
                Modifier.size(52.dp).background(c.accentSoft, RoundedCornerShape(VynRadius.lg)),
                contentAlignment = Alignment.Center,
            ) { VynIconView(VynIcon.Monitor, tint = c.accent, size = 28.dp) }
            Column(Modifier.weight(1f)) {
                VynText(item.name, style = t.title, maxLines = 1)
                VynText(item.primaryHost + ":" + item.tcpPort, style = t.caption, color = c.textSecondary)
            }
        }
        Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
            if (item.os.isNotBlank()) Detail(stringResource(Res.string.pc_system), item.os.replaceFirstChar { it.uppercase() })
            Detail(stringResource(Res.string.pc_address), item.hosts.joinToString(", ").ifBlank { "—" })
            if (!item.isManual) Detail(stringResource(Res.string.pc_slots), slotsText(item.freeSlots))
            if (item.fingerprint.isNotBlank()) Detail(stringResource(Res.string.pc_fingerprint), item.fingerprint.chunked(4).joinToString(" "))
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                VynText(stringResource(Res.string.pc_found_via), style = t.body, color = c.textSecondary)
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    item.via.sorted().forEach { via ->
                        VynBadge(
                            stringResource(
                                when (via) {
                                    DiscoveryVia.Mdns -> Res.string.via_mdns
                                    DiscoveryVia.Broadcast -> Res.string.via_broadcast
                                    DiscoveryVia.Manual -> Res.string.via_manual
                                },
                            ),
                            BadgeTone.Neutral,
                        )
                    }
                }
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            VynButton(stringResource(Res.string.action_close), onDismiss, kind = ButtonKind.Secondary, modifier = Modifier.weight(1f))
            when {
                connected -> VynButton(stringResource(Res.string.session_disconnect), onDisconnect, modifier = Modifier.weight(1f))
                paired -> VynButton(stringResource(Res.string.session_connect), { onConnect(item) }, modifier = Modifier.weight(1f))
                else -> VynButton(stringResource(Res.string.pc_pair), { onPair(item) }, modifier = Modifier.weight(1f))
            }
        }
        if (paired) {
            VynButton(stringResource(Res.string.pc_forget), { onForget(item) }, kind = ButtonKind.Destructive, modifier = Modifier.fillMaxWidth())
        }
    }
}

@Composable
private fun Detail(label: String, value: String) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        VynText(label, style = t.body, color = c.textSecondary)
        VynText(value, style = t.bodyStrong, maxLines = 2, modifier = Modifier.padding(start = 16.dp))
    }
}

/** Entrada manual de `IP[:porta]`, para quando a descoberta não acha o PC. */
@Composable
fun ManualPcSheet(visible: Boolean, onDismiss: () -> Unit, onAdd: (String) -> Boolean) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    val haptics = LocalHaptics.current
    var text by remember { mutableStateOf("") }
    var invalid by remember { mutableStateOf(false) }
    val focus = remember { FocusRequester() }

    LaunchedEffect(visible) {
        if (visible) {
            text = ""
            invalid = false
            runCatching { focus.requestFocus() }
        }
    }

    fun submit() {
        if (parseHostPort(text) != null && onAdd(text)) {
            haptics.confirm()
        } else {
            invalid = true
            haptics.reject()
        }
    }

    VynSheet(visible = visible, onDismiss = onDismiss) {
        VynText(stringResource(Res.string.manual_title), style = t.title)
        BasicTextField(
            value = text,
            onValueChange = {
                text = it.filter { ch -> ch.isDigit() || ch == '.' || ch == ':' }.take(21)
                invalid = false
            },
            singleLine = true,
            textStyle = t.subtitle.copy(color = c.textPrimary),
            cursorBrush = SolidColor(c.accent),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal, imeAction = ImeAction.Done),
            keyboardActions = KeyboardActions(onDone = { submit() }),
            modifier = Modifier.fillMaxWidth().focusRequester(focus),
            decorationBox = { inner ->
                Box(
                    Modifier
                        .fillMaxWidth()
                        .background(c.control, RoundedCornerShape(VynRadius.lg))
                        .padding(horizontal = 16.dp, vertical = 15.dp),
                ) {
                    if (text.isEmpty()) VynText("192.168.0.10", style = t.subtitle, color = c.textTertiary)
                    inner()
                }
            },
        )
        VynText(
            stringResource(if (invalid) Res.string.manual_error else Res.string.manual_hint),
            style = t.caption,
            color = if (invalid) c.danger else c.textSecondary,
        )
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            VynButton(stringResource(Res.string.action_close), onDismiss, kind = ButtonKind.Secondary, modifier = Modifier.weight(1f))
            VynButton(stringResource(Res.string.manual_add), onClick = ::submit, modifier = Modifier.weight(1f))
        }
    }
}
