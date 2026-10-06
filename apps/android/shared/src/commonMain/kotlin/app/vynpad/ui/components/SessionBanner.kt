package app.vynpad.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import app.vynpad.pairing.SessionEnd
import app.vynpad.pairing.SessionState
import app.vynpad.resources.Res
import app.vynpad.resources.action_ok
import app.vynpad.resources.session_connected
import app.vynpad.resources.session_connecting
import app.vynpad.resources.session_disconnect
import app.vynpad.resources.session_ended_kicked
import app.vynpad.resources.session_ended_lost
import app.vynpad.resources.session_ended_revoked
import app.vynpad.resources.session_reconnecting
import app.vynpad.resources.session_slot
import app.vynpad.ui.theme.VynTheme
import org.jetbrains.compose.resources.stringResource

/**
 * Faixa de estado da conexão no topo da tela Conectar: conectado (com o jogador na cor do
 * slot), conectando/reconectando (com anéis pulsando) ou o motivo de a sessão ter terminado.
 */
@Composable
fun SessionBanner(state: SessionState, onDisconnect: () -> Unit, modifier: Modifier = Modifier) {
    if (state is SessionState.Idle) return
    val c = VynTheme.colors
    val t = VynTheme.typography
    val slotColor = (state as? SessionState.Connected)?.let { c.slot(it.slot) }
    GlassCard(modifier.fillMaxWidth(), radius = 20.dp, glow = slotColor) {
        Row(
            Modifier.fillMaxWidth().padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            Box(Modifier.size(44.dp), contentAlignment = Alignment.Center) {
                when (state) {
                    is SessionState.Connected -> PulseRings(Modifier.size(44.dp), color = slotColor ?: c.accent)
                    is SessionState.Connecting, is SessionState.Reconnecting -> PulseRings(Modifier.size(44.dp), color = c.warning)
                    else -> VynIconView(VynIcon.Monitor, tint = c.danger, size = 26.dp)
                }
            }
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                when (state) {
                    is SessionState.Connected -> {
                        VynText(stringResource(Res.string.session_connected, state.serverName), style = t.bodyStrong, maxLines = 1)
                        VynText(stringResource(Res.string.session_slot, state.slot + 1), style = t.caption, color = slotColor ?: c.accent)
                    }
                    is SessionState.Connecting ->
                        VynText(stringResource(Res.string.session_connecting, state.serverName), style = t.bodyStrong, maxLines = 2)
                    is SessionState.Reconnecting ->
                        VynText(stringResource(Res.string.session_reconnecting, state.serverName), style = t.bodyStrong, maxLines = 2)
                    is SessionState.Ended -> VynText(
                        stringResource(
                            when (state.reason) {
                                SessionEnd.Kicked -> Res.string.session_ended_kicked
                                SessionEnd.Revoked -> Res.string.session_ended_revoked
                                SessionEnd.Lost -> Res.string.session_ended_lost
                            },
                            state.serverName,
                        ),
                        style = t.body,
                        maxLines = 3,
                    )
                    SessionState.Idle -> Unit
                }
            }
            VynButton(
                text = stringResource(if (state is SessionState.Ended) Res.string.action_ok else Res.string.session_disconnect),
                onClick = onDisconnect,
                kind = ButtonKind.Secondary,
            )
        }
    }
}
