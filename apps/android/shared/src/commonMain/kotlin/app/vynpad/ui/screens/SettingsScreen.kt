package app.vynpad.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import app.vynpad.core.loadCoreInfo
import app.vynpad.input.RumbleStrength
import app.vynpad.resources.Res
import app.vynpad.resources.pad_color_fixed
import app.vynpad.resources.pad_color_slot
import app.vynpad.resources.rumble_medium
import app.vynpad.resources.rumble_soft
import app.vynpad.resources.rumble_strong
import app.vynpad.resources.settings_about
import app.vynpad.resources.settings_appearance
import app.vynpad.resources.settings_core
import app.vynpad.resources.settings_feedback
import app.vynpad.resources.settings_haptics
import app.vynpad.resources.settings_haptics_desc
import app.vynpad.resources.settings_haptics_strength
import app.vynpad.resources.settings_pad_color
import app.vynpad.resources.settings_rumble
import app.vynpad.resources.settings_rumble_desc
import app.vynpad.resources.settings_rumble_strength
import app.vynpad.resources.settings_rumble_test
import app.vynpad.resources.settings_theme
import app.vynpad.resources.settings_title
import app.vynpad.resources.settings_version
import app.vynpad.resources.theme_dark
import app.vynpad.resources.theme_light
import app.vynpad.resources.theme_system
import app.vynpad.settings.AppSettings
import app.vynpad.ui.components.ButtonKind
import app.vynpad.ui.components.GlassCard
import app.vynpad.ui.components.VynButton
import app.vynpad.ui.components.VynSegmented
import app.vynpad.ui.components.VynText
import app.vynpad.ui.components.VynToggle
import app.vynpad.ui.components.entrance
import app.vynpad.ui.haptics.HapticStrength
import app.vynpad.ui.theme.ThemeMode
import app.vynpad.ui.theme.VynTheme
import org.jetbrains.compose.resources.stringResource

const val AppVersion = "0.1.0"

@Composable
fun SettingsScreen(settings: AppSettings, onSettings: (AppSettings) -> Unit, onTestRumble: () -> Unit, modifier: Modifier = Modifier) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    val themeMode = settings.theme
    val rumble = settings.rumble
    val coreInfo = remember { loadCoreInfo() }
    val coreSummary = coreInfo?.let { "${it.version} · v${it.protocolVersion}" } ?: "—"
    Column(
        modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp),
    ) {
        Spacer(Modifier.height(16.dp))
        VynText(stringResource(Res.string.settings_title), style = t.hero, modifier = Modifier.entrance(0))

        Section(Res.string.settings_appearance, Modifier.entrance(1)) {
            VynText(stringResource(Res.string.settings_theme), style = t.bodyStrong)
            VynSegmented(
                options = listOf(
                    ThemeMode.System to stringResource(Res.string.theme_system),
                    ThemeMode.Dark to stringResource(Res.string.theme_dark),
                    ThemeMode.Light to stringResource(Res.string.theme_light),
                ),
                selected = themeMode,
                onSelect = { onSettings(settings.copy(theme = it)) },
            )
            VynText(stringResource(Res.string.settings_pad_color), style = t.bodyStrong)
            VynSegmented(
                options = listOf(
                    true to stringResource(Res.string.pad_color_slot),
                    false to stringResource(Res.string.pad_color_fixed),
                ),
                selected = settings.slotAccent,
                onSelect = { onSettings(settings.copy(slotAccent = it)) },
            )
        }

        Section(Res.string.settings_feedback, Modifier.entrance(2)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    VynText(stringResource(Res.string.settings_haptics), style = t.bodyStrong)
                    VynText(stringResource(Res.string.settings_haptics_desc), style = t.caption, color = c.textSecondary)
                }
                VynToggle(settings.hapticsEnabled, { onSettings(settings.copy(hapticsEnabled = it)) })
            }
            if (settings.hapticsEnabled) {
                VynText(stringResource(Res.string.settings_haptics_strength), style = t.bodyStrong)
                VynSegmented(
                    options = listOf(
                        HapticStrength.Soft to stringResource(Res.string.rumble_soft),
                        HapticStrength.Medium to stringResource(Res.string.rumble_medium),
                        HapticStrength.Strong to stringResource(Res.string.rumble_strong),
                    ),
                    selected = settings.hapticStrength,
                    onSelect = { onSettings(settings.copy(hapticStrength = it)) },
                )
            }
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    VynText(stringResource(Res.string.settings_rumble), style = t.bodyStrong)
                    VynText(stringResource(Res.string.settings_rumble_desc), style = t.caption, color = c.textSecondary)
                }
                VynToggle(rumble.enabled, { onSettings(settings.withRumble(rumble.copy(enabled = it))) })
            }
            if (rumble.enabled) {
                VynText(stringResource(Res.string.settings_rumble_strength), style = t.bodyStrong)
                VynSegmented(
                    options = listOf(
                        RumbleStrength.Soft to stringResource(Res.string.rumble_soft),
                        RumbleStrength.Medium to stringResource(Res.string.rumble_medium),
                        RumbleStrength.Strong to stringResource(Res.string.rumble_strong),
                    ),
                    selected = rumble.strength,
                    onSelect = { onSettings(settings.withRumble(rumble.copy(strength = it))) },
                )
                VynButton(
                    stringResource(Res.string.settings_rumble_test),
                    onTestRumble,
                    kind = ButtonKind.Secondary,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }

        Section(Res.string.settings_about, Modifier.entrance(3)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                VynText(stringResource(Res.string.settings_version), style = t.body, color = c.textSecondary)
                VynText(AppVersion, style = t.bodyStrong)
            }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                VynText(stringResource(Res.string.settings_core), style = t.body, color = c.textSecondary)
                VynText(coreSummary, style = t.bodyStrong)
            }
        }
    }
}

@Composable
private fun Section(title: org.jetbrains.compose.resources.StringResource, modifier: Modifier = Modifier, content: @Composable () -> Unit) {
    val c = VynTheme.colors
    Column(modifier, verticalArrangement = Arrangement.spacedBy(8.dp)) {
        VynText(
            stringResource(title).uppercase(),
            style = VynTheme.typography.caption,
            color = c.textTertiary,
            modifier = Modifier.padding(start = 6.dp),
        )
        GlassCard(Modifier.fillMaxWidth()) {
            Column(Modifier.fillMaxWidth().padding(18.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                content()
            }
        }
    }
}
