package app.vynpad.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.unit.dp
import app.vynpad.resources.Res
import app.vynpad.resources.skin_blur
import app.vynpad.resources.skin_choose
import app.vynpad.resources.skin_contrast_fix
import app.vynpad.resources.skin_contrast_warn
import app.vynpad.resources.skin_hint
import app.vynpad.resources.skin_import_error
import app.vynpad.resources.skin_pad_opacity
import app.vynpad.resources.skin_remove
import app.vynpad.resources.skin_title
import app.vynpad.resources.skin_use_color
import app.vynpad.resources.skin_veil
import app.vynpad.skin.SkinBitmap
import app.vynpad.skin.rememberSkinPicker
import app.vynpad.ui.components.ButtonKind
import app.vynpad.ui.components.VynButton
import app.vynpad.ui.components.VynSlider
import app.vynpad.ui.components.VynText
import app.vynpad.ui.components.VynToggle
import app.vynpad.ui.gamepad.GamepadGeometry
import app.vynpad.ui.gamepad.PadLayout
import app.vynpad.ui.gamepad.PadSkin
import app.vynpad.ui.gamepad.PadSkins
import app.vynpad.ui.gamepad.SkinContrast
import app.vynpad.ui.theme.VynTheme
import org.jetbrains.compose.resources.stringResource

/**
 * O conteúdo do painel "Visual do jogo": imagem de fundo, véu, desfoque, cor da imagem, opacidade
 * dos botões e o aviso de legibilidade. Mexe só no [PadSkin] do layout em edição.
 */
@Composable
fun SkinPanelContent(
    draft: PadLayout,
    background: SkinBitmap?,
    /** Largura ÷ altura da tela, para medir o contraste nas posições reais das peças. */
    aspect: Float,
    onDraft: (PadLayout) -> Unit,
    onCommit: (PadLayout) -> Unit,
) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    val skin = draft.skin
    var importFailed by remember { mutableStateOf(false) }

    val pick = rememberSkinPicker { imported ->
        importFailed = imported == null
        if (imported != null) {
            // Mantém o que o usuário já ajustou (véu, opacidade); só troca a imagem e a cor.
            val next = draft.copy(skin = skin.copy(image = imported.file, accent = imported.accent))
            onDraft(next)
            onCommit(next)
        }
    }

    fun update(next: PadSkin, commit: Boolean = true) {
        val layout = draft.copy(skin = next)
        onDraft(layout)
        if (commit) onCommit(layout)
    }

    VynText(stringResource(Res.string.skin_title), style = t.bodyStrong, maxLines = 1)
    if (!skin.hasImage) {
        VynText(stringResource(Res.string.skin_hint), style = t.caption, color = c.textSecondary)
    }
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        VynButton(stringResource(Res.string.skin_choose), pick, kind = ButtonKind.Secondary, modifier = Modifier.weight(1f))
        if (skin.hasImage) {
            VynButton(
                stringResource(Res.string.skin_remove),
                { update(skin.copy(image = null, accent = null)) },
                kind = ButtonKind.Destructive,
                modifier = Modifier.weight(1f),
            )
        }
    }
    if (importFailed) VynText(stringResource(Res.string.skin_import_error), style = t.caption, color = c.danger)

    if (!skin.hasImage) return

    VynText(stringResource(Res.string.skin_veil) + " · " + (skin.veil * 100).toInt() + "%", style = t.caption, color = c.textSecondary)
    VynSlider(
        value = skin.veil,
        onValueChange = { update(skin.copy(veil = it), commit = false) },
        valueRange = 0f..PadSkins.MAX_VEIL,
        onValueChangeFinished = { onCommit(draft) },
    )

    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        VynText(stringResource(Res.string.skin_blur), style = t.body, modifier = Modifier.weight(1f))
        VynToggle(skin.blur, { update(skin.copy(blur = it)) })
    }

    if (skin.accent != null) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Box(Modifier.size(14.dp).background(Color(skin.accent.toInt()), CircleShape))
            VynText(stringResource(Res.string.skin_use_color), style = t.body, modifier = Modifier.weight(1f))
            VynToggle(skin.useImageAccent, { update(skin.copy(useImageAccent = it)) })
        }
    }

    VynText(
        stringResource(Res.string.skin_pad_opacity) + " · " + (skin.padOpacity * 100).toInt() + "%",
        style = t.caption,
        color = c.textSecondary,
    )
    VynSlider(
        value = skin.padOpacity,
        onValueChange = { update(skin.copy(padOpacity = it), commit = false) },
        valueRange = PadSkins.MIN_PAD_OPACITY..1f,
        onValueChangeFinished = { onCommit(draft) },
    )

    // Aviso de legibilidade: mede o contraste do corpo dos botões com a imagem sob cada peça.
    if (background != null) {
        val geometry = remember(draft, aspect) { GamepadGeometry(aspect * 1000f, 1000f, draft) }
        val fillLuma = (c.padTop.luminance() + c.padBottom.luminance()) / 2f
        if (!SkinContrast.isLegible(background.luma, geometry, skin.veil, fillLuma)) {
            VynText(stringResource(Res.string.skin_contrast_warn), style = t.caption, color = c.warning)
            VynButton(
                stringResource(Res.string.skin_contrast_fix),
                { update(skin.copy(veil = SkinContrast.autoVeil(background.luma, geometry, skin.veil, fillLuma))) },
                kind = ButtonKind.Secondary,
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}
