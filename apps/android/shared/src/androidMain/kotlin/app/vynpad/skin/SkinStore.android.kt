package app.vynpad.skin

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import app.vynpad.ui.gamepad.AccentPicker
import app.vynpad.ui.gamepad.LumaGrid
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.ByteArrayInputStream
import java.io.File
import java.io.InputStream
import java.util.UUID

/** Imagem importada: no máximo este lado maior, para a memória não estourar numa tela pequena ou enorme. */
private const val MIN_LONG_SIDE = 1280
private const val MAX_LONG_SIDE = 2560
private const val JPEG_QUALITY = 88

private const val GC_GRACE_MILLIS = 120_000L

/** O desfoque é feito reduzindo a imagem por este fator e ampliando de volta (uma vez, na importação). */
private const val BLUR_FACTOR = 14

/** Amostra pequena para a cor de destaque. */
private const val ACCENT_SAMPLE_SIDE = 64

internal class AndroidSkinStore(context: Context) : SkinStore {
    private val appContext = context.applicationContext
    private val dir = File(appContext.filesDir, "skins").apply { mkdirs() }

    private fun fileOf(name: String, blur: Boolean) = File(dir, if (blur) "${name}_b.jpg" else "$name.jpg")

    /** Lado maior da tela em pixels: a imagem é guardada neste tamanho (e nunca maior que o limite). */
    private fun targetLongSide(): Int {
        val m = appContext.resources.displayMetrics
        return maxOf(m.widthPixels, m.heightPixels).coerceIn(MIN_LONG_SIDE, MAX_LONG_SIDE)
    }

    override suspend fun load(file: String, blur: Boolean): SkinBitmap? = withContext(Dispatchers.IO) {
        val f = fileOf(file, blur).takeIf { it.exists() } ?: fileOf(file, false).takeIf { it.exists() } ?: return@withContext null
        val bitmap = runCatching { BitmapFactory.decodeFile(f.path) }.getOrNull() ?: return@withContext null
        val grid = lumaOf(bitmap)
        SkinBitmap(bitmap.asImageBitmap(), grid)
    }

    override fun gc(keep: Set<String>) {
        val now = System.currentTimeMillis()
        dir.listFiles()?.forEach { f ->
            val base = f.name.removeSuffix(".jpg").removeSuffix("_b")
            // Arquivo recém-criado pode ainda não ter chegado aos ajustes: só apaga os com mais de 2 min.
            if (base !in keep && now - f.lastModified() > GC_GRACE_MILLIS) f.delete()
        }
    }

    /** Lê o que o usuário escolheu, reduz, guarda (com a versão desfocada) e extrai a cor. */
    suspend fun import(uri: Uri): ImportedSkin? = import { appContext.contentResolver.openInputStream(uri) }

    /** Importa uma imagem que veio embutida num layout compartilhado. */
    suspend fun importBytes(bytes: ByteArray): ImportedSkin? = import { ByteArrayInputStream(bytes) }

    /** Os bytes da imagem guardada (para embutir num layout exportado). */
    fun readImage(file: String): ByteArray? = fileOf(file, false).takeIf { it.exists() }?.readBytes()

    private suspend fun import(open: () -> InputStream?): ImportedSkin? = withContext(Dispatchers.IO) {
        runCatching {
            val target = targetLongSide()
            val decoded = decodeScaled(open, target) ?: return@runCatching null
            val name = "s" + UUID.randomUUID().toString().replace("-", "").take(12)
            save(decoded, fileOf(name, false))
            save(blurred(decoded), fileOf(name, true))
            val accent = AccentPicker.pick(sample(decoded))
            decoded.recycle()
            ImportedSkin(name, accent)
        }.getOrNull()
    }

    /** Decodifica já reduzido: primeiro só as medidas, depois com o menor `inSampleSize` que não fica abaixo do alvo. */
    private fun decodeScaled(open: () -> InputStream?, target: Int): Bitmap? {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        open()?.use { BitmapFactory.decodeStream(it, null, bounds) }
        val longSide = maxOf(bounds.outWidth, bounds.outHeight)
        if (longSide <= 0) return null
        var sample = 1
        while (longSide / (sample * 2) >= target) sample *= 2
        val options = BitmapFactory.Options().apply { inSampleSize = sample }
        val bitmap = open()?.use { BitmapFactory.decodeStream(it, null, options) } ?: return null
        val side = maxOf(bitmap.width, bitmap.height)
        if (side <= target) return bitmap
        val k = target.toFloat() / side
        val scaled = Bitmap.createScaledBitmap(bitmap, (bitmap.width * k).toInt().coerceAtLeast(1), (bitmap.height * k).toInt().coerceAtLeast(1), true)
        if (scaled !== bitmap) bitmap.recycle()
        return scaled
    }

    private fun save(bitmap: Bitmap, file: File) {
        file.outputStream().use { bitmap.compress(Bitmap.CompressFormat.JPEG, JPEG_QUALITY, it) }
    }

    /** Desfoque barato e feito uma vez: reduz bastante e amplia com filtro bilinear. */
    private fun blurred(src: Bitmap): Bitmap {
        val small = Bitmap.createScaledBitmap(src, (src.width / BLUR_FACTOR).coerceAtLeast(2), (src.height / BLUR_FACTOR).coerceAtLeast(2), true)
        val big = Bitmap.createScaledBitmap(small, src.width, src.height, true)
        if (small !== big) small.recycle()
        return big
    }

    private fun sample(src: Bitmap): IntArray {
        val k = ACCENT_SAMPLE_SIDE.toFloat() / maxOf(src.width, src.height)
        val w = (src.width * k).toInt().coerceAtLeast(1)
        val h = (src.height * k).toInt().coerceAtLeast(1)
        val small = Bitmap.createScaledBitmap(src, w, h, true)
        val pixels = IntArray(w * h)
        small.getPixels(pixels, 0, w, 0, 0, w, h)
        if (small !== src) small.recycle()
        return pixels
    }

    /**
     * A grade de luminância cobre a tela como a imagem é desenhada (corte central até a proporção
     * 16:9 da tela em paisagem), então a amostra usa só esse recorte.
     */
    private fun lumaOf(src: Bitmap): LumaGrid {
        val k = 96f / maxOf(src.width, src.height)
        val w = (src.width * k).toInt().coerceAtLeast(1)
        val h = (src.height * k).toInt().coerceAtLeast(1)
        val small = Bitmap.createScaledBitmap(src, w, h, true)
        val pixels = IntArray(w * h)
        small.getPixels(pixels, 0, w, 0, 0, w, h)
        if (small !== src) small.recycle()
        val m = appContext.resources.displayMetrics
        val screenAspect = maxOf(m.widthPixels, m.heightPixels).toFloat() / minOf(m.widthPixels, m.heightPixels)
        return LumaGrid.from(cropToAspect(pixels, w, h, screenAspect), cropW(w, h, screenAspect), cropH(w, h, screenAspect))
    }

    private fun cropW(w: Int, h: Int, aspect: Float) = if (w.toFloat() / h > aspect) (h * aspect).toInt().coerceAtLeast(1) else w

    private fun cropH(w: Int, h: Int, aspect: Float) = if (w.toFloat() / h > aspect) h else (w / aspect).toInt().coerceAtLeast(1)

    private fun cropToAspect(pixels: IntArray, w: Int, h: Int, aspect: Float): IntArray {
        val cw = cropW(w, h, aspect)
        val ch = cropH(w, h, aspect)
        if (cw == w && ch == h) return pixels
        val x0 = (w - cw) / 2
        val y0 = (h - ch) / 2
        return IntArray(cw * ch) { i -> pixels[(y0 + i / cw) * w + x0 + i % cw] }
    }
}

@Composable
actual fun rememberSkinStore(): SkinStore {
    val context = LocalContext.current
    return remember(context) { AndroidSkinStore(context) }
}

@Composable
actual fun rememberSkinPicker(onImported: (ImportedSkin?) -> Unit): () -> Unit {
    val context = LocalContext.current
    val store = remember(context) { AndroidSkinStore(context) }
    val scope = rememberCoroutineScope()
    val callback by rememberUpdatedState(onImported)
    val launcher = rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
        if (uri != null) scope.launch { callback(store.import(uri)) }
    }
    return remember(launcher) { { launcher.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly)) } }
}
