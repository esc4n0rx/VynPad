package app.vynpad.skin

import androidx.compose.runtime.Composable
import app.vynpad.ui.gamepad.PadLayout

/**
 * Devolve uma função que abre o seletor de destino do sistema e grava o layout (com a imagem de
 * fundo embutida) num arquivo `.vynlayout`. [onResult] recebe `true` se gravou, `false` se falhou;
 * cancelar não chama nada.
 */
@Composable
expect fun rememberLayoutExport(onResult: (Boolean) -> Unit): (PadLayout) -> Unit

/**
 * Devolve uma função que abre o seletor de arquivos e lê um `.vynlayout`. [onResult] recebe o
 * layout já validado (a imagem, se houver, já guardada no aparelho) ou `null` se o arquivo não é
 * um layout do VynPad. Cancelar não chama nada.
 */
@Composable
expect fun rememberLayoutImport(onResult: (PadLayout?) -> Unit): () -> Unit
