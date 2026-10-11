package app.vynpad.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import app.vynpad.resources.Res
import app.vynpad.resources.el_back
import app.vynpad.resources.el_custom
import app.vynpad.resources.el_dpad
import app.vynpad.resources.el_face
import app.vynpad.resources.el_guide
import app.vynpad.resources.el_lb
import app.vynpad.resources.el_left_stick
import app.vynpad.resources.el_lt
import app.vynpad.resources.el_rb
import app.vynpad.resources.el_right_stick
import app.vynpad.resources.el_rt
import app.vynpad.resources.el_start
import app.vynpad.resources.layout_delete
import app.vynpad.resources.layout_done
import app.vynpad.resources.layout_export
import app.vynpad.resources.layout_export_error
import app.vynpad.resources.layout_exported
import app.vynpad.resources.layout_hint
import app.vynpad.resources.layout_import
import app.vynpad.resources.layout_import_error
import app.vynpad.resources.layout_imported
import app.vynpad.resources.layout_models
import app.vynpad.resources.layout_my_layouts
import app.vynpad.resources.layout_name_hint
import app.vynpad.resources.layout_new
import app.vynpad.resources.layout_opacity
import app.vynpad.resources.layout_presets
import app.vynpad.resources.layout_rename
import app.vynpad.resources.layout_reset_part
import app.vynpad.resources.layout_restore
import app.vynpad.resources.layout_save
import app.vynpad.resources.layout_size
import app.vynpad.resources.layout_snap
import app.vynpad.resources.layout_visible
import app.vynpad.resources.preset_compact
import app.vynpad.resources.preset_default
import app.vynpad.resources.preset_dpad
import app.vynpad.resources.preset_left
import app.vynpad.resources.skin_open
import app.vynpad.resources.stick_deadzone
import app.vynpad.resources.stick_fixed
import app.vynpad.resources.stick_floating
import app.vynpad.resources.stick_mode
import app.vynpad.resources.stick_travel
import app.vynpad.settings.AppSettings
import app.vynpad.skin.SkinBitmap
import app.vynpad.skin.rememberLayoutExport
import app.vynpad.skin.rememberLayoutImport
import app.vynpad.ui.components.ButtonKind
import app.vynpad.ui.components.GlassCard
import app.vynpad.ui.components.VynButton
import app.vynpad.ui.components.VynChip
import app.vynpad.ui.components.VynSegmented
import app.vynpad.ui.components.VynSheet
import app.vynpad.ui.components.VynSlider
import app.vynpad.ui.components.VynText
import app.vynpad.ui.components.VynTextField
import app.vynpad.ui.components.VynToggle
import app.vynpad.ui.gamepad.PadEditing
import app.vynpad.ui.gamepad.PadIds
import app.vynpad.ui.gamepad.PadLayout
import app.vynpad.ui.gamepad.PadLayouts
import app.vynpad.ui.gamepad.PadPreset
import app.vynpad.ui.gamepad.PadWidget
import app.vynpad.ui.gamepad.StickConfig
import app.vynpad.ui.gamepad.WidgetKind
import app.vynpad.ui.theme.VynTheme
import org.jetbrains.compose.resources.StringResource
import org.jetbrains.compose.resources.stringResource

/** O nome da peça no editor: as do controle padrão têm o seu; as demais, um nome genérico. */
fun PadWidget.label(): StringResource = when (id) {
    PadIds.LeftStick -> Res.string.el_left_stick
    PadIds.RightStick -> Res.string.el_right_stick
    PadIds.Dpad -> Res.string.el_dpad
    PadIds.Face -> Res.string.el_face
    PadIds.Back -> Res.string.el_back
    PadIds.Guide -> Res.string.el_guide
    PadIds.Start -> Res.string.el_start
    PadIds.LT -> Res.string.el_lt
    PadIds.LB -> Res.string.el_lb
    PadIds.RB -> Res.string.el_rb
    PadIds.RT -> Res.string.el_rt
    else -> Res.string.el_custom
}

private fun PadPreset.label(): StringResource = when (this) {
    PadPreset.Default -> Res.string.preset_default
    PadPreset.Compact -> Res.string.preset_compact
    PadPreset.LeftHanded -> Res.string.preset_left
    PadPreset.DpadOnly -> Res.string.preset_dpad
}

/**
 * Os controles do editor de layout, por cima do gamepad: a barra do topo (concluir, modelos,
 * ímã), o painel de ajuste da peça escolhida (do lado oposto a ela, para ela ficar à vista) e a
 * folha de layouts e modelos.
 *
 * [draft] é o layout em edição; [onDraft] o atualiza a cada passo (arrastar um controle) e
 * [onCommit] o grava (ao soltar). Tudo o mais que mexe nos ajustes sai por [onSettings].
 */
@Composable
fun LayoutEditorChrome(
    settings: AppSettings,
    draft: PadLayout,
    selected: String?,
    snap: Boolean,
    background: SkinBitmap?,
    aspect: Float,
    onSnap: (Boolean) -> Unit,
    onDraft: (PadLayout) -> Unit,
    onCommit: (PadLayout) -> Unit,
    onSettings: (AppSettings) -> Unit,
    onDone: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    val selectedWidget = selected?.let(draft::widget)
    var modelsOpen by remember { mutableStateOf(false) }
    var skinOpen by remember { mutableStateOf(false) }
    // Escolher uma peça troca o painel do visual pelo painel dela.
    LaunchedEffect(selected) { if (selected != null) skinOpen = false }

    Box(modifier) {
        // ---- Barra do topo
        GlassCard(
            Modifier.align(Alignment.TopCenter).padding(top = 10.dp),
            radius = 28.dp,
            base = c.windowSolid.copy(alpha = 0.85f),
        ) {
            Column(
                Modifier.padding(horizontal = 14.dp, vertical = 8.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    VynButton(stringResource(Res.string.layout_done), onDone)
                    VynButton(stringResource(Res.string.layout_models), { modelsOpen = true }, kind = ButtonKind.Secondary)
                    VynButton(stringResource(Res.string.skin_open), { skinOpen = !skinOpen }, kind = ButtonKind.Secondary)
                    VynText(stringResource(Res.string.layout_snap), style = t.caption, color = c.textSecondary)
                    VynToggle(snap, onSnap)
                }
                VynText(
                    if (selectedWidget != null) stringResource(selectedWidget.label()) else stringResource(Res.string.layout_hint),
                    style = t.caption,
                    color = c.textSecondary,
                    maxLines = 1,
                )
            }
        }

        // ---- Painel da peça escolhida
        if (selected != null && selectedWidget != null) {
            val spec = selectedWidget.spec
            val onRight = spec.x < 0.5f
            BoxWithConstraints(Modifier.fillMaxSize()) {
                GlassCard(
                    Modifier
                        .align(if (onRight) Alignment.CenterEnd else Alignment.CenterStart)
                        .padding(horizontal = 12.dp)
                        .width(250.dp)
                        // Os analógicos têm mais ajustes: o painel rola em vez de passar da tela.
                        .heightIn(max = maxHeight - 76.dp),
                    radius = 22.dp,
                    base = c.windowSolid.copy(alpha = 0.88f),
                ) {
                    Column(Modifier.verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        VynText(stringResource(selectedWidget.label()), style = t.bodyStrong, maxLines = 1)
                        VynText(stringResource(Res.string.layout_size), style = t.caption, color = c.textSecondary)
                        VynSlider(
                            value = spec.scale,
                            onValueChange = { onDraft(PadEditing.setScale(draft, selected, it)) },
                            valueRange = PadLayouts.MIN_SCALE..PadLayouts.MAX_SCALE,
                            onValueChangeFinished = { onCommit(draft) },
                        )
                        if (selectedWidget.kind == WidgetKind.Stick) {
                            val stick = selectedWidget.stick
                            VynText(stringResource(Res.string.stick_mode), style = t.caption, color = c.textSecondary)
                            VynSegmented(
                                options = listOf(
                                    true to stringResource(Res.string.stick_floating),
                                    false to stringResource(Res.string.stick_fixed),
                                ),
                                selected = stick.floating,
                                onSelect = {
                                    val next = PadEditing.setStick(draft, selected, stick.copy(floating = it))
                                    onDraft(next)
                                    onCommit(next)
                                },
                            )
                            VynText(stringResource(Res.string.stick_travel), style = t.caption, color = c.textSecondary)
                            VynSlider(
                                value = stick.travelScale,
                                onValueChange = { onDraft(PadEditing.setStick(draft, selected, stick.copy(travelScale = it))) },
                                valueRange = PadLayouts.MIN_TRAVEL..PadLayouts.MAX_TRAVEL,
                                onValueChangeFinished = { onCommit(draft) },
                            )
                            VynText(
                                stringResource(Res.string.stick_deadzone) + " · " + (stick.deadzone * 100).toInt() + "%",
                                style = t.caption,
                                color = c.textSecondary,
                            )
                            VynSlider(
                                value = stick.deadzone,
                                onValueChange = { onDraft(PadEditing.setStick(draft, selected, stick.copy(deadzone = it))) },
                                valueRange = 0f..PadLayouts.MAX_DEADZONE,
                                onValueChangeFinished = { onCommit(draft) },
                            )
                        }
                        VynText(stringResource(Res.string.layout_opacity), style = t.caption, color = c.textSecondary)
                        VynSlider(
                            value = spec.opacity,
                            onValueChange = { onDraft(PadEditing.setOpacity(draft, selected, it)) },
                            valueRange = PadLayouts.MIN_OPACITY..1f,
                            onValueChangeFinished = { onCommit(draft) },
                        )
                        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                            VynText(stringResource(Res.string.layout_visible), style = t.body, modifier = Modifier.weight(1f))
                            VynToggle(spec.visible, {
                                val next = PadEditing.setVisible(draft, selected, it)
                                onDraft(next)
                                onCommit(next)
                            })
                        }
                        VynButton(
                            stringResource(Res.string.layout_reset_part),
                            {
                                val next = PadEditing.resetElement(draft, selected)
                                onDraft(next)
                                onCommit(next)
                            },
                            kind = ButtonKind.Secondary,
                            modifier = Modifier.fillMaxWidth(),
                        )
                    }
                }
            }
        }

        if (skinOpen && selected == null) {
            BoxWithConstraints(Modifier.fillMaxSize()) {
                GlassCard(
                    Modifier
                        .align(Alignment.CenterEnd)
                        .padding(horizontal = 12.dp)
                        .width(270.dp)
                        .heightIn(max = maxHeight - 76.dp),
                    radius = 22.dp,
                    base = c.windowSolid.copy(alpha = 0.88f),
                ) {
                    Column(
                        Modifier.verticalScroll(rememberScrollState()).padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        SkinPanelContent(draft, background, aspect, onDraft, onCommit)
                    }
                }
            }
        }

        ModelsSheet(
            visible = modelsOpen,
            settings = settings,
            draft = draft,
            onSettings = onSettings,
            onDraft = onDraft,
            onCommit = onCommit,
            onDismiss = { modelsOpen = false },
        )
    }
}

@Composable
private fun ModelsSheet(
    visible: Boolean,
    settings: AppSettings,
    draft: PadLayout,
    onSettings: (AppSettings) -> Unit,
    onDraft: (PadLayout) -> Unit,
    onCommit: (PadLayout) -> Unit,
    onDismiss: () -> Unit,
) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    var renaming by remember(visible) { mutableStateOf<String?>(null) }
    var confirmDelete by remember(visible, settings.activeLayoutId) { mutableStateOf(false) }
    // Resultado da última exportação/importação, mostrado na própria folha.
    var transfer by remember(visible) { mutableStateOf<StringResource?>(null) }
    val export = rememberLayoutExport { ok ->
        transfer = if (ok) Res.string.layout_exported else Res.string.layout_export_error
    }
    val import = rememberLayoutImport { layout ->
        if (layout != null) onSettings(settings.addImported(layout))
        transfer = if (layout != null) Res.string.layout_imported else Res.string.layout_import_error
    }

    VynSheet(visible = visible, onDismiss = onDismiss, maxWidth = 520.dp) {
        VynText(stringResource(Res.string.layout_models), style = t.title)

        VynText(stringResource(Res.string.layout_my_layouts), style = t.caption, color = c.textSecondary)
        Row(
            Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            settings.layouts.forEach { layout ->
                VynChip(layout.name, layout.id == settings.activeLayoutId, { onSettings(settings.select(layout.id)) })
            }
        }

        val editing = renaming
        if (editing != null) {
            val save = {
                onSettings(settings.rename(draft.id, editing))
                renaming = null
            }
            VynTextField(
                value = editing,
                onValueChange = { renaming = it.take(AppSettings.MAX_NAME) },
                placeholder = stringResource(Res.string.layout_name_hint),
                onDone = save,
            )
            VynButton(stringResource(Res.string.layout_save), save, modifier = Modifier.fillMaxWidth())
        } else {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                VynButton(
                    stringResource(Res.string.layout_new),
                    { onSettings(settings.duplicate(draft)) },
                    kind = ButtonKind.Secondary,
                    modifier = Modifier.weight(1f),
                )
                VynButton(
                    stringResource(Res.string.layout_rename),
                    { renaming = draft.name },
                    kind = ButtonKind.Secondary,
                    modifier = Modifier.weight(1f),
                )
                if (settings.layouts.size > 1) {
                    VynButton(
                        stringResource(Res.string.layout_delete) + if (confirmDelete) "?" else "",
                        {
                            if (confirmDelete) onSettings(settings.remove(draft.id)) else confirmDelete = true
                        },
                        kind = ButtonKind.Destructive,
                        modifier = Modifier.weight(1f),
                    )
                }
            }
        }

        VynText(stringResource(Res.string.layout_presets), style = t.caption, color = c.textSecondary)
        val apply = { preset: PadPreset ->
            val next = PadLayouts.fromPreset(preset, draft.id, draft.name).copy(skin = draft.skin)
            onDraft(next)
            onCommit(next)
        }
        PadPreset.entries.chunked(2).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEach { preset ->
                    VynButton(
                        stringResource(preset.label()),
                        { apply(preset) },
                        kind = ButtonKind.Secondary,
                        modifier = Modifier.weight(1f),
                    )
                }
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            VynButton(
                stringResource(Res.string.layout_export),
                { export(draft) },
                kind = ButtonKind.Secondary,
                modifier = Modifier.weight(1f),
            )
            VynButton(
                stringResource(Res.string.layout_import),
                import,
                kind = ButtonKind.Secondary,
                modifier = Modifier.weight(1f),
            )
        }
        transfer?.let {
            val failed = it == Res.string.layout_export_error || it == Res.string.layout_import_error
            VynText(stringResource(it), style = t.caption, color = if (failed) c.danger else c.success)
        }
        VynButton(
            stringResource(Res.string.layout_restore),
            { apply(PadPreset.Default) },
            kind = ButtonKind.Ghost,
            modifier = Modifier.fillMaxWidth(),
        )
    }
}
