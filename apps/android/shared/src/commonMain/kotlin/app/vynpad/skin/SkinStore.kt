package app.vynpad.skin

import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.ImageBitmap
import app.vynpad.ui.gamepad.LumaGrid

/** A imagem de fundo pronta para desenhar e a grade de luminância dela (para o contraste). */
class SkinBitmap(val image: ImageBitmap, val luma: LumaGrid)

/** Uma imagem recém-importada: o nome do arquivo guardado e a cor de destaque tirada dela. */
class ImportedSkin(val file: String, val accent: Long?)

/** Onde ficam as imagens de fundo: só no aparelho, nunca enviadas para fora. */
interface SkinStore {
    /** Lê a imagem (ou a versão desfocada); `null` se o arquivo sumiu ou está corrompido. */
    suspend fun load(file: String, blur: Boolean): SkinBitmap?

    /** Apaga as imagens que nenhum layout usa mais. */
    fun gc(keep: Set<String>)
}

@Composable
expect fun rememberSkinStore(): SkinStore

/**
 * Devolve uma função que abre o seletor de fotos do sistema (sem pedir permissão de
 * armazenamento). Ao escolher, a imagem é reduzida ao tamanho da tela e guardada; [onImported]
 * recebe o resultado, ou `null` se a imagem não pôde ser lida. Cancelar não chama nada.
 */
@Composable
expect fun rememberSkinPicker(onImported: (ImportedSkin?) -> Unit): () -> Unit
