package app.vynpad.ui.components

import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextOverflow
import app.vynpad.ui.theme.VynTheme

/** Texto do design system: cor e fonte vêm do tema, não do Material. */
@Composable
fun VynText(
    text: String,
    modifier: Modifier = Modifier,
    style: TextStyle = VynTheme.typography.body,
    color: Color = Color.Unspecified,
    maxLines: Int = Int.MAX_VALUE,
) {
    val resolved = if (color == Color.Unspecified) style.copy(color = if (style.color == Color.Unspecified) VynTheme.colors.textPrimary else style.color) else style.copy(color = color)
    BasicText(text, modifier, resolved, maxLines = maxLines, overflow = TextOverflow.Ellipsis)
}
