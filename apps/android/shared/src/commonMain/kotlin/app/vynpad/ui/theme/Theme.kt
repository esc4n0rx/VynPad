package app.vynpad.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

/**
 * Tokens do design system (docs/00-visao-geral/DESIGN-SYSTEM.md), os mesmos valores do client
 * Windows (apps/desktop/src/index.css). Cor só entra por aqui: nos componentes, nada de hex.
 */
@Immutable
data class VynColors(
    val isDark: Boolean,
    val accent: Color,
    val accentSoft: Color,
    val onAccent: Color,
    val indigo: Color,
    val windowSolid: Color,
    val elevated: Color,
    val control: Color,
    val controlHover: Color,
    val overlay: Color,
    val separator: Color,
    val shine: Color,
    val textPrimary: Color,
    val textSecondary: Color,
    val textTertiary: Color,
    val success: Color,
    val warning: Color,
    val danger: Color,
    val info: Color,
    val padTop: Color,
    val padBottom: Color,
    val padEdge: Color,
    val padSurface: Color,
    val padWell: Color,
    val padGlyph: Color,
) {
    /** Cor de cada slot de jogador (P1–P4). */
    fun slot(index: Int): Color = SlotColors[index.coerceIn(0, SlotColors.lastIndex)]
}

val SlotColors = listOf(
    Color(0xFF0A84FF),
    Color(0xFF30D158),
    Color(0xFFFF9F0A),
    Color(0xFFBF5AF2),
)

private val Accent = Color(0xFF0A84FF)

val DarkColors = VynColors(
    isDark = true,
    accent = Accent,
    accentSoft = Accent.copy(alpha = 0.18f),
    onAccent = Color.White,
    indigo = Color(0xFF5E5CE6),
    windowSolid = Color(0xFF1C1C1E),
    elevated = Color(0x8C3A3A3C),
    control = Color(0x3D767680),
    controlHover = Color(0x5C767680),
    overlay = Color(0x66000000),
    separator = Color(0x73545458),
    shine = Color(0x0FFFFFFF),
    textPrimary = Color(0xEBFFFFFF),
    textSecondary = Color(0x99EBEBF5),
    textTertiary = Color(0x4DEBEBF5),
    success = Color(0xFF30D158),
    warning = Color(0xFFFFD60A),
    danger = Color(0xFFFF453A),
    info = Color(0xFF64D2FF),
    padTop = Color(0xFF56565C),
    padBottom = Color(0xFF28282C),
    padEdge = Color(0x29FFFFFF),
    padSurface = Color(0xFF1E1E22),
    padWell = Color(0xBF141418),
    padGlyph = Color(0x8CEBEBF5),
)

val LightColors = VynColors(
    isDark = false,
    accent = Accent,
    accentSoft = Accent.copy(alpha = 0.18f),
    onAccent = Color.White,
    indigo = Color(0xFF5E5CE6),
    windowSolid = Color(0xFFF6F6F6),
    elevated = Color(0xB3FFFFFF),
    control = Color(0x1F767680),
    controlHover = Color(0x33767680),
    overlay = Color(0x40000000),
    separator = Color(0x333C3C43),
    shine = Color(0xCCFFFFFF),
    textPrimary = Color(0xE0000000),
    textSecondary = Color(0xA63C3C43),
    textTertiary = Color(0x593C3C43),
    success = Color(0xFF1F8A3A),
    warning = Color(0xFF9A6700),
    danger = Color(0xFFD70015),
    info = Color(0xFF0071A4),
    padTop = Color(0xFFFCFCFD),
    padBottom = Color(0xFFD6D6DE),
    padEdge = Color(0x2E000000),
    padSurface = Color(0xFFECECF2),
    padWell = Color(0x333C3C43),
    padGlyph = Color(0x993C3C43),
)

/** Raios do design system: sm 6 · md 10 · lg 14 · xl 20. */
object VynRadius {
    val sm = 6.dp
    val md = 10.dp
    val lg = 14.dp
    val xl = 20.dp
}

enum class ThemeMode { System, Dark, Light }

val LocalVynColors = staticCompositionLocalOf { DarkColors }

object VynTheme {
    val colors: VynColors
        @Composable
        @ReadOnlyComposable
        get() = LocalVynColors.current

    val typography: VynTypography
        @Composable
        @ReadOnlyComposable
        get() = LocalVynTypography.current
}

@Composable
fun VynTheme(
    mode: ThemeMode = ThemeMode.Dark,
    content: @Composable () -> Unit,
) {
    val dark = when (mode) {
        ThemeMode.System -> isSystemInDarkTheme()
        ThemeMode.Dark -> true
        ThemeMode.Light -> false
    }
    CompositionLocalProvider(
        LocalVynColors provides if (dark) DarkColors else LightColors,
        LocalVynTypography provides rememberVynTypography(),
        content = content,
    )
}
