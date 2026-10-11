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
import app.vynpad.resources.el_button
import app.vynpad.resources.el_custom
import app.vynpad.resources.el_dpad
import app.vynpad.resources.el_face
import app.vynpad.resources.el_guide
import app.vynpad.resources.el_lb
import app.vynpad.resources.el_left_stick
import app.vynpad.resources.el_lt
import app.vynpad.resources.el_pedal
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
import app.vynpad.resources.part_action
import app.vynpad.resources.part_add
import app.vynpad.resources.part_add_button
import app.vynpad.resources.part_add_pedal_left
import app.vynpad.resources.part_add_pedal_right
import app.vynpad.resources.part_add_title
import app.vynpad.resources.part_axis
import app.vynpad.resources.part_duplicate
import app.vynpad.resources.part_latch
import app.vynpad.resources.part_limit
import app.vynpad.resources.part_remove
import app.vynpad.resources.part_style
import app.vynpad.resources.part_text
import app.vynpad.resources.part_text_hint
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
import app.vynpad.resources.style_large
import app.vynpad.resources.style_pill
import app.vynpad.resources.style_round
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
import app.vynpad.ui.gamepad.ButtonStyle
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

    else -> when (kind) {
        WidgetKind.Button -> Res.string.el_button
        WidgetKind.Pedal -> Res.string.el_pedal
        else -> Res.string.el_custom
    }
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
    onSelect: (String?) -> Unit,
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
    var addOpen by remember { mutableStateOf(false) }
    var skinOpen by remember { mutableStateOf(false) }
    var confirmRemove by remember(selected) { mutableStateOf(false) }
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
                    VynButton(stringResource(Res.string.part_add), { addOpen = true }, kind = ButtonKind.Secondary)
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
                        if (selectedWidget.kind == WidgetKind.Button) {
                            ButtonOptions(selectedWidget, draft, onDraft, onCommit)
                        }
                        if (selectedWidget.kind == WidgetKind.Pedal) {
                            VynText(stringResource(Res.string.part_axis), style = t.caption, color = c.textSecondary)
                            VynSegmented(
                                options = listOf(true to "LT", false to "RT"),
                                selected = selectedWidget.left,
                                onSelect = {
                                    val next = PadEditing.setSide(draft, selected, it)
                                    onDraft(next)
                                    onCommit(next)
                                },
                            )
                        }
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
                        if (selected in PadIds.BuiltIn) {
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
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            VynButton(
                                stringResource(Res.string.part_duplicate),
                                {
                                    PadEditing.duplicate(draft, selected)?.let { (next, id) ->
                                        onDraft(next)
                                        onCommit(next)
                                        onSelect(id)
                                    }
                                },
                                kind = ButtonKind.Secondary,
                                modifier = Modifier.weight(1f),
                            )
                            VynButton(
                                stringResource(Res.string.part_remove) + if (confirmRemove) "?" else "",
                                {
                                    if (confirmRemove) {
                                        val next = PadEditing.remove(draft, selected)
                                        onDraft(next)
                                        onCommit(next)
                                        onSelect(null)
                                    } else {
                                        confirmRemove = true
                                    }
                                },
                                kind = ButtonKind.Destructive,
                                modifier = Modifier.weight(1f),
                            )
                        }
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

        AddPartSheet(
            visible = addOpen,
            draft = draft,
            onAdd = { template ->
                val added = PadEditing.add(draft, template)
                if (added != null) {
                    onDraft(added.first)
                    onCommit(added.first)
                    onSelect(added.second)
                }
                added != null
            },
            onDismiss = { addOpen = false },
        )

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

/** O que um botão livre aciona, como se parece, o texto dele e se trava ao toque. */
@Composable
private fun ButtonOptions(widget: PadWidget, draft: PadLayout, onDraft: (PadLayout) -> Unit, onCommit: (PadLayout) -> Unit) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    fun apply(next: PadLayout) {
        onDraft(next)
        onCommit(next)
    }
    VynText(stringResource(Res.string.part_action), style = t.caption, color = c.textSecondary)
    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        PadLayouts.ButtonMasks.forEach { mask ->
            VynChip(PadLayouts.buttonName(mask), widget.mask == mask, { apply(PadEditing.setMask(draft, widget.id, mask)) })
        }
    }
    VynText(stringResource(Res.string.part_style), style = t.caption, color = c.textSecondary)
    VynSegmented(
        options = listOf(
            ButtonStyle.Round to stringResource(Res.string.style_round),
            ButtonStyle.RoundLarge to stringResource(Res.string.style_large),
            ButtonStyle.Pill to stringResource(Res.string.style_pill),
        ),
        selected = widget.style,
        onSelect = { apply(PadEditing.setStyle(draft, widget.id, it)) },
    )
    VynText(stringResource(Res.string.part_text), style = t.caption, color = c.textSecondary)
    VynTextField(
        value = widget.label,
        onValueChange = { onDraft(PadEditing.setLabel(draft, widget.id, it)) },
        placeholder = stringResource(Res.string.part_text_hint),
        onDone = { onCommit(draft) },
    )
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        VynText(stringResource(Res.string.part_latch), style = t.body, modifier = Modifier.weight(1f))
        VynToggle(widget.latch, { apply(PadEditing.setLatch(draft, widget.id, it)) })
    }
}

/** A folha para acrescentar uma peça nova ao layout. */
@Composable
private fun AddPartSheet(visible: Boolean, draft: PadLayout, onAdd: (PadWidget) -> Boolean, onDismiss: () -> Unit) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    var full by remember(visible) { mutableStateOf(false) }
    val add = { template: PadWidget ->
        if (onAdd(template)) onDismiss() else full = true
    }
    VynSheet(visible = visible, onDismiss = onDismiss, maxWidth = 420.dp) {
        VynText(stringResource(Res.string.part_add_title), style = t.title)
        VynButton(stringResource(Res.string.part_add_button), { add(PadLayouts.newButton()) }, kind = ButtonKind.Secondary, modifier = Modifier.fillMaxWidth())
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            VynButton(stringResource(Res.string.part_add_pedal_left), {
                add(PadLayouts.newPedal(left = true))
            }, kind = ButtonKind.Secondary, modifier = Modifier.weight(1f))
            VynButton(stringResource(Res.string.part_add_pedal_right), {
                add(PadLayouts.newPedal(left = false))
            }, kind = ButtonKind.Secondary, modifier = Modifier.weight(1f))
        }
        if (full || draft.widgets.size >= PadLayouts.MAX_WIDGETS) {
            VynText(stringResource(Res.string.part_limit), style = t.caption, color = c.danger)
        }
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
