package app.vynpad.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import app.vynpad.ui.theme.vynSpring

/**
 * Folha de vidro que sobe do rodapé sobre um véu escuro. Sobe e desce com a mola padrão e
 * fecha ao tocar fora. Quem chama trata o botão voltar do sistema.
 */
@Composable
fun VynSheet(
    visible: Boolean,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
    /** Largura máxima do cartão (em paisagem uma folha de tela inteira ficaria desproporcional). */
    maxWidth: Dp = 560.dp,
    content: @Composable ColumnScope.() -> Unit,
) {
    val c = app.vynpad.ui.theme.VynTheme.colors
    Box(modifier.fillMaxSize()) {
        AnimatedVisibility(visible, enter = fadeIn(tween(180)), exit = fadeOut(tween(180))) {
            Box(
                Modifier
                    .fillMaxSize()
                    .background(c.overlay)
                    .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null, onClick = onDismiss),
            )
        }
        AnimatedVisibility(
            visible,
            modifier = Modifier.align(Alignment.BottomCenter),
            enter = slideInVertically(vynSpring()) { it } + fadeIn(tween(120)),
            exit = slideOutVertically(tween(200)) { it } + fadeOut(tween(160)),
        ) {
            Box(
                Modifier
                    .windowInsetsPadding(WindowInsets.safeDrawing)
                    .padding(horizontal = 12.dp, vertical = 12.dp),
            ) {
                GlassCard(
                    Modifier
                        .widthIn(max = maxWidth)
                        .fillMaxWidth()
                        // Engole os toques: tocar no cartão não fecha a folha.
                        .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {},
                    radius = 28.dp,
                    // Base quase opaca: sem blur, o texto do fundo vazaria pelo vidro.
                    base = c.windowSolid.copy(alpha = 0.97f),
                ) {
                    Column(
                        Modifier.fillMaxWidth().padding(22.dp),
                        verticalArrangement = Arrangement.spacedBy(14.dp),
                        content = content,
                    )
                }
            }
        }
    }
}
