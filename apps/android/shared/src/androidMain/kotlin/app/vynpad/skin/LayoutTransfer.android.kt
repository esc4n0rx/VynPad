package app.vynpad.skin

import android.content.Context
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import app.vynpad.ui.gamepad.LayoutPackage
import app.vynpad.ui.gamepad.PadLayout
import java.io.ByteArrayOutputStream
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** Extensão sugerida ao salvar; o seletor de importação aceita qualquer arquivo e valida o conteúdo. */
private const val EXTENSION = "vynlayout"

@Composable
actual fun rememberLayoutExport(onResult: (Boolean) -> Unit): (PadLayout) -> Unit {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val callback by rememberUpdatedState(onResult)
    var pending by remember { mutableStateOf<PadLayout?>(null) }
    val launcher = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/octet-stream")) { uri ->
        val layout = pending
        pending = null
        if (uri != null && layout != null) scope.launch { callback(writeLayout(context, uri, layout)) }
    }
    return remember(launcher) {
        { layout ->
            pending = layout
            launcher.launch(layout.name.filter { it.isLetterOrDigit() || it == ' ' || it == '-' }.trim().ifEmpty { "layout" } + ".$EXTENSION")
        }
    }
}

@Composable
actual fun rememberLayoutImport(onResult: (PadLayout?) -> Unit): () -> Unit {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val callback by rememberUpdatedState(onResult)
    val launcher = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) scope.launch { callback(readLayout(context, uri)) }
    }
    return remember(launcher) { { launcher.launch(arrayOf("*/*")) } }
}

private suspend fun writeLayout(context: Context, uri: Uri, layout: PadLayout): Boolean = withContext(Dispatchers.IO) {
    runCatching {
        val image = layout.skin.image?.let { AndroidSkinStore(context).readImage(it) }
        val text = LayoutPackage.encode(layout, image)
        context.contentResolver.openOutputStream(uri, "wt")?.use { it.write(text.toByteArray(Charsets.UTF_8)) } != null
    }.getOrDefault(false)
}

private suspend fun readLayout(context: Context, uri: Uri): PadLayout? = withContext(Dispatchers.IO) {
    runCatching {
        val bytes = context.contentResolver.openInputStream(uri)?.use { readLimited(it, LayoutPackage.MAX_BYTES) } ?: return@runCatching null
        val pkg = LayoutPackage.decode(bytes.toString(Charsets.UTF_8)) ?: return@runCatching null
        val imageBytes = LayoutPackage.imageBytes(pkg)
        if (imageBytes == null) {
            pkg.layout
        } else {
            // Imagem que não decodifica não derruba o layout: ele entra sem fundo.
            val imported = AndroidSkinStore(context).importBytes(imageBytes)
            if (imported == null) {
                pkg.layout
            } else {
                pkg.layout.copy(skin = pkg.layout.skin.copy(image = imported.file, accent = imported.accent ?: pkg.layout.skin.accent))
            }
        }
    }.getOrNull()
}

/** Lê no máximo [limit] bytes; arquivo maior que isso vira erro (nunca é carregado inteiro). */
private fun readLimited(input: java.io.InputStream, limit: Int): ByteArray {
    val out = ByteArrayOutputStream()
    val buffer = ByteArray(64 * 1024)
    while (true) {
        val n = input.read(buffer)
        if (n < 0) break
        out.write(buffer, 0, n)
        require(out.size() <= limit) { "arquivo grande demais" }
    }
    return out.toByteArray()
}
