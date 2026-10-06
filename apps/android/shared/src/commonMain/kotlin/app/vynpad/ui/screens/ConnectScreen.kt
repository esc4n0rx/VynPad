package app.vynpad.ui.screens

import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import app.vynpad.discovery.DiscoveredPc
import app.vynpad.discovery.ScanStatus
import app.vynpad.resources.Res
import app.vynpad.resources.app_name
import app.vynpad.resources.connect_add_ip
import app.vynpad.resources.connect_empty_body
import app.vynpad.resources.connect_empty_title
import app.vynpad.resources.connect_found_body
import app.vynpad.resources.connect_found_title
import app.vynpad.resources.connect_list_title
import app.vynpad.resources.connect_preview_gamepad
import app.vynpad.resources.connect_scanning_body
import app.vynpad.resources.connect_scanning_title
import app.vynpad.resources.connect_search
import app.vynpad.resources.connect_search_again
import app.vynpad.resources.connect_searching
import app.vynpad.resources.connect_tagline
import app.vynpad.resources.pc_manual
import app.vynpad.resources.pc_new
import app.vynpad.resources.pc_slots_free
import app.vynpad.resources.pc_slots_full
import app.vynpad.resources.status_found
import app.vynpad.resources.status_offline
import app.vynpad.resources.status_searching
import app.vynpad.ui.components.BadgeTone
import app.vynpad.ui.components.ButtonKind
import app.vynpad.ui.components.DiscoveryRadar
import app.vynpad.ui.components.GlassCard
import app.vynpad.ui.components.VynBadge
import app.vynpad.ui.components.VynButton
import app.vynpad.ui.components.VynIcon
import app.vynpad.ui.components.VynIconView
import app.vynpad.ui.components.VynText
import app.vynpad.ui.components.entrance
import app.vynpad.ui.haptics.LocalHaptics
import app.vynpad.ui.theme.VynRadius
import app.vynpad.ui.theme.VynTheme
import org.jetbrains.compose.resources.stringResource

@Composable
fun ConnectScreen(
    pcs: List<DiscoveredPc>,
    status: ScanStatus,
    onSearch: () -> Unit,
    onSelectPc: (DiscoveredPc) -> Unit,
    onAddByIp: () -> Unit,
    onPreviewGamepad: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    val scanning = status == ScanStatus.Scanning
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
                Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 22.dp).animateContentSize(),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                DiscoveryRadar(pcs = pcs, scanning = scanning, onSelect = onSelectPc, modifier = Modifier.padding(horizontal = 6.dp))

                VynBadge(
                    text = stringResource(
                        when {
                            scanning -> Res.string.status_searching
                            pcs.isNotEmpty() -> Res.string.status_found
                            else -> Res.string.status_offline
                        },
                    ),
                    tone = when {
                        scanning -> BadgeTone.Accent
                        pcs.isNotEmpty() -> BadgeTone.Success
                        else -> BadgeTone.Neutral
                    },
                )
                VynText(
                    when {
                        pcs.isNotEmpty() -> stringResource(Res.string.connect_found_title, pcs.size)
                        scanning -> stringResource(Res.string.connect_scanning_title)
                        else -> stringResource(Res.string.connect_empty_title)
                    },
                    style = t.title,
                )
                VynText(
                    when {
                        pcs.isNotEmpty() -> stringResource(Res.string.connect_found_body)
                        scanning -> stringResource(Res.string.connect_scanning_body)
                        else -> stringResource(Res.string.connect_empty_body)
                    },
                    style = t.body.copy(textAlign = TextAlign.Center),
                    color = c.textSecondary,
                )
                Spacer(Modifier.height(2.dp))
                VynButton(
                    text = stringResource(
                        when {
                            scanning -> Res.string.connect_searching
                            pcs.isEmpty() && status == ScanStatus.Idle -> Res.string.connect_search
                            else -> Res.string.connect_search_again
                        },
                    ),
                    onClick = onSearch,
                    enabled = !scanning,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }

        if (pcs.isNotEmpty()) {
            Column(Modifier.entrance(2), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                VynText(
                    stringResource(Res.string.connect_list_title).uppercase(),
                    style = t.caption,
                    color = c.textTertiary,
                    modifier = Modifier.padding(start = 6.dp),
                )
                pcs.forEach { pc -> PcRow(pc, onClick = { onSelectPc(pc) }) }
            }
        }

        Row(Modifier.fillMaxWidth().entrance(3), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            VynButton(
                text = stringResource(Res.string.connect_add_ip),
                onClick = onAddByIp,
                kind = ButtonKind.Secondary,
                modifier = Modifier.weight(1f),
            )
            VynButton(
                text = stringResource(Res.string.connect_preview_gamepad),
                onClick = onPreviewGamepad,
                kind = ButtonKind.Secondary,
                modifier = Modifier.weight(1f),
            )
        }
    }
}

@Composable
private fun PcRow(pc: DiscoveredPc, onClick: () -> Unit) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    val haptics = LocalHaptics.current
    GlassCard(
        Modifier
            .fillMaxWidth()
            .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {
                haptics.tick()
                onClick()
            },
        radius = VynRadius.xl,
    ) {
        Row(
            Modifier.fillMaxWidth().padding(14.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            Box(
                Modifier.size(46.dp).background(c.accentSoft, RoundedCornerShape(VynRadius.lg)),
                contentAlignment = Alignment.Center,
            ) {
                VynIconView(VynIcon.Monitor, tint = c.accent, size = 24.dp)
            }
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                VynText(pc.name, style = t.bodyStrong, maxLines = 1)
                VynText(
                    if (pc.isManual) pc.primaryHost else "${pc.primaryHost} · " + slotsText(pc.freeSlots),
                    style = t.caption,
                    color = c.textSecondary,
                    maxLines = 1,
                )
            }
            VynBadge(
                text = stringResource(if (pc.isManual) Res.string.pc_manual else Res.string.pc_new),
                tone = if (pc.isManual) BadgeTone.Neutral else BadgeTone.Info,
            )
        }
    }
}

@Composable
internal fun slotsText(free: Int): String =
    if (free > 0) stringResource(Res.string.pc_slots_free, free) else stringResource(Res.string.pc_slots_full)
