package app.vynpad.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.shadow.Shadow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.dp
import app.vynpad.ui.theme.VynRadius
import app.vynpad.ui.theme.VynTheme

/**
 * Fundo do app: o "material" que no desktop vem do Mica. No celular não há material do sistema,
 * então é o sólido do tema com dois halos suaves (accent e índigo) que dão profundidade ao vidro.
 */
@Composable
fun VynBackground(modifier: Modifier = Modifier, content: @Composable BoxScope.() -> Unit) {
    val c = VynTheme.colors
    Box(
        modifier
            .fillMaxSize()
            .background(c.windowSolid)
            .drawBehind {
                val strong = if (c.isDark) 0.22f else 0.12f
                drawRect(
                    Brush.radialGradient(
                        colors = listOf(c.accent.copy(alpha = strong), Color.Transparent),
                        center = Offset(size.width * 0.05f, -size.height * 0.02f),
                        radius = size.maxDimension * 0.7f,
                    ),
                )
                drawRect(
                    Brush.radialGradient(
                        colors = listOf(c.indigo.copy(alpha = strong * 0.7f), Color.Transparent),
                        center = Offset(size.width * 1.0f, size.height * 1.02f),
                        radius = size.maxDimension * 0.6f,
                    ),
                )
            },
        content = content,
    )
}

/**
 * Card de vidro: camada `elevated` translúcida, brilho de 1 px no topo (o `inset` do
 * `--shadow-card`) e sombra suave.
 */
@Composable
fun GlassCard(
    modifier: Modifier = Modifier,
    radius: Dp = VynRadius.xl,
    glow: Color? = null,
    content: @Composable BoxScope.() -> Unit,
) {
    val c = VynTheme.colors
    val shape = RoundedCornerShape(radius)
    Box(
        modifier
            // Sombra só por fora do contorno: a de elevação vazaria pelo vidro translúcido.
            .dropShadow(
                shape,
                Shadow(
                    radius = 18.dp,
                    color = (glow ?: Color.Black).copy(alpha = if (glow != null) 0.45f else if (c.isDark) 0.35f else 0.12f),
                    offset = DpOffset(0.dp, 8.dp),
                ),
            )
            .clip(shape)
            .background(c.elevated)
            .background(Brush.verticalGradient(listOf(c.shine, Color.Transparent), endY = 160f))
            .border(
                width = 1.dp,
                brush = Brush.verticalGradient(listOf(c.shine.copy(alpha = if (c.isDark) 0.18f else 0.9f), c.separator.copy(alpha = 0.35f))),
                shape = shape,
            ),
        content = content,
    )
}
