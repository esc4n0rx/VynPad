package app.vynpad.input

import kotlin.concurrent.Volatile

/** Máscaras de botões no formato `XInput` (as mesmas de `Buttons` em `vynpad-protocol`). */
object PadButtons {
    const val DPAD_UP = 0x0001
    const val DPAD_DOWN = 0x0002
    const val DPAD_LEFT = 0x0004
    const val DPAD_RIGHT = 0x0008
    const val START = 0x0010
    const val BACK = 0x0020
    const val LEFT_THUMB = 0x0040
    const val RIGHT_THUMB = 0x0080
    const val LEFT_SHOULDER = 0x0100
    const val RIGHT_SHOULDER = 0x0200
    const val GUIDE = 0x0400
    const val A = 0x1000
    const val B = 0x2000
    const val X = 0x4000
    const val Y = 0x8000

    const val DPAD_ALL = DPAD_UP or DPAD_DOWN or DPAD_LEFT or DPAD_RIGHT
}

/** Uma leitura consistente do controle, pronta para virar um pacote `InputState`. */
data class GamepadSnapshot(
    val buttons: Int,
    /** Gatilho esquerdo, 0–255. */
    val lt: Int,
    /** Gatilho direito, 0–255. */
    val rt: Int,
    /** Analógicos de −32768 a 32767; `y` positivo = cima (convenção XInput). */
    val lx: Int,
    val ly: Int,
    val rx: Int,
    val ry: Int,
)

/** Quem quer saber, na hora, quando o controle muda (o motor de envio). */
fun interface GamepadSink {
    fun onChanged(buttons: Int, lt: Int, rt: Int, lx: Int, ly: Int, rx: Int, ry: Int)
}

/**
 * O estado atual do controle, numa estrutura mutável única: a tela escreve a cada toque e o
 * motor de envio lê na taxa dele, sem passar pela recomposição do Compose.
 *
 * Cada campo é `@Volatile` (a escrita vem da thread principal, a leitura de outra). Os campos
 * são independentes, então uma leitura pode pegar um analógico um toque à frente do botão: um
 * pacote a 250 Hz corrige isso no seguinte.
 */
class GamepadState {
    /** Recebe cada mudança logo depois de gravada; sem alocação. */
    @Volatile var sink: GamepadSink? = null

    @Volatile var buttons: Int = 0

    @Volatile var lt: Int = 0

    @Volatile var rt: Int = 0

    @Volatile var lx: Int = 0

    @Volatile var ly: Int = 0

    @Volatile var rx: Int = 0

    @Volatile var ry: Int = 0

    fun snapshot() = GamepadSnapshot(buttons, lt, rt, lx, ly, rx, ry)

    /** Avisa o [sink] de que os campos acabaram de ser atualizados. */
    fun commit() {
        sink?.onChanged(buttons, lt, rt, lx, ly, rx, ry)
    }

    /** Solta tudo (ao sair da tela ou perder o foco). */
    fun reset() {
        buttons = 0
        lt = 0
        rt = 0
        lx = 0
        ly = 0
        rx = 0
        ry = 0
        commit()
    }
}
