package app.vynpad.ui.theme

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import app.vynpad.resources.Res
import app.vynpad.resources.inter_variable
import org.jetbrains.compose.resources.Font

/**
 * Escala do design system: 11 caption · 13 corpo · 15 subtítulo · 20 título · 28 hero.
 * Pesos 400/500/600; fonte Inter (a mesma do client Windows).
 */
@Immutable
data class VynTypography(
    val caption: TextStyle,
    val body: TextStyle,
    val bodyStrong: TextStyle,
    val subtitle: TextStyle,
    val title: TextStyle,
    val hero: TextStyle,
)

val LocalVynTypography = staticCompositionLocalOf<VynTypography> {
    error("VynTheme não foi fornecido")
}

@Composable
internal fun rememberVynTypography(): VynTypography {
    val regular = Font(Res.font.inter_variable, FontWeight.Normal, variationSettings = FontVariation.Settings(FontVariation.weight(400)))
    val medium = Font(Res.font.inter_variable, FontWeight.Medium, variationSettings = FontVariation.Settings(FontVariation.weight(500)))
    val semi = Font(Res.font.inter_variable, FontWeight.SemiBold, variationSettings = FontVariation.Settings(FontVariation.weight(600)))
    val family = FontFamily(regular, medium, semi)
    return remember(family) {
        fun style(size: Int, weight: FontWeight, line: Int, tracking: Double = 0.0) = TextStyle(
            fontFamily = family,
            fontSize = size.sp,
            fontWeight = weight,
            lineHeight = line.sp,
            letterSpacing = tracking.sp,
            fontFeatureSettings = "cv11, ss01, tnum",
        )
        VynTypography(
            caption = style(11, FontWeight.Medium, 14, 0.2),
            body = style(14, FontWeight.Normal, 20),
            bodyStrong = style(14, FontWeight.Medium, 20),
            subtitle = style(16, FontWeight.SemiBold, 22),
            title = style(22, FontWeight.SemiBold, 28, -0.3),
            hero = style(30, FontWeight.SemiBold, 36, -0.6),
        )
    }
}
