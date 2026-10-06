package app.vynpad.pairing

import app.vynpad.input.GamepadState
import app.vynpad.input.RumbleSettings
import kotlinx.coroutines.flow.StateFlow

/** Por que a sessão acabou sem o usuário pedir. */
enum class SessionEnd {
    /** O PC desconectou este celular (pela interface do VynPad). */
    Kicked,

    /** O PC removeu este celular: é preciso parear de novo. */
    Revoked,

    /** Não foi possível reconectar. */
    Lost,
}

/** Estado da conexão com um PC pareado. */
sealed interface SessionState {
    data object Idle : SessionState

    data class Connecting(val serverName: String) : SessionState

    /** Conectado: o PC mostra este celular no [slot] (0–3). */
    data class Connected(val serverId: String, val serverName: String, val slot: Int) : SessionState

    /** A conexão caiu e o app tenta voltar (0,5 s → 1 → 2 → 4 → 5 s, como no protocolo). */
    data class Reconnecting(val serverName: String, val attempt: Int) : SessionState

    data class Ended(val serverName: String, val reason: SessionEnd) : SessionState
}

/** A conexão (canal de controle e, depois, o input) com um PC pareado. */
interface SessionManager {
    val state: StateFlow<SessionState>

    /** Latência de rede até o PC (suavizada), em µs; nula sem sessão ou antes do primeiro pong. */
    val rttMicros: StateFlow<Int?>

    /** `true` enquanto os sensores de movimento estão ligados (o PC pediu gyro neste controle). */
    val gyroActive: StateFlow<Boolean>

    /** `true` enquanto o gyro mede o viés: o usuário deve segurar o celular parado. */
    val gyroCalibrating: StateFlow<Boolean>

    /** Mede o viés do gyro de novo (recentraliza); o celular precisa ficar parado por ~2 s. */
    fun recenterGyro()

    /** Passa a enviar ao PC tudo o que mudar em [state] (o controle na tela). */
    fun bindInput(state: GamepadState)

    /** Ajustes da vibração que vem do jogo; valem já, mesmo no meio de uma vibração. */
    fun setRumble(settings: RumbleSettings)

    /** Vibra o celular uma vez com os ajustes atuais, para o usuário calibrar. */
    fun testRumble()

    /** Conecta a um PC já pareado com o token do dispositivo. */
    fun connect(server: PairedServer)

    /** Encerra a conexão (ou dispensa o aviso de uma sessão que terminou). */
    fun disconnect()
}
