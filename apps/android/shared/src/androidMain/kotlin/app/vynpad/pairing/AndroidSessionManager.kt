package app.vynpad.pairing

import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import uniffi.vynpad_client_core.ConfigInfo
import uniffi.vynpad_client_core.ControlConnection
import uniffi.vynpad_client_core.CoreException
import uniffi.vynpad_client_core.Credential
import uniffi.vynpad_client_core.DeviceInfo
import uniffi.vynpad_client_core.SessionListener
import uniffi.vynpad_client_core.TrustPolicy

private const val TAG = "VynSession"

/** Pausas entre tentativas de reconexão (PROTOCOLO §5.3): 0,5 s, 1 s, 2 s, 4 s e depois 5 s. */
private val Backoff = longArrayOf(500, 1_000, 2_000, 4_000, 5_000)

/** Desiste de reconectar depois deste tempo sem conseguir. */
private const val GIVE_UP_AFTER_MILLIS = 120_000L

/**
 * Mantém a sessão de controle com um PC pareado: conecta com o `device_token`, responde ao
 * keepalive (pelo núcleo Rust) e reconecta sozinho se a rede cair.
 */
internal class AndroidSessionManager(
    private val deviceInfo: () -> DeviceInfo,
    /** Chamado quando o PC removeu este celular: o pareamento não vale mais. */
    private val onRevoked: (serverId: String) -> Unit,
) : SessionManager {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val _state = MutableStateFlow<SessionState>(SessionState.Idle)
    override val state: StateFlow<SessionState> = _state.asStateFlow()

    private val lock = Any()
    private var connection: ControlConnection? = null
    private var target: PairedServer? = null
    private var job: Job? = null

    /** Adota uma conexão que acabou de parear (é a mesma sessão: o PC não vê um desconecta-conecta). */
    fun adopt(conn: ControlConnection, server: PairedServer) {
        endCurrent()
        synchronized(lock) { target = server }
        attach(conn, server)
    }

    override fun connect(server: PairedServer) {
        endCurrent()
        synchronized(lock) { target = server }
        _state.value = SessionState.Connecting(server.name)
        job = scope.launch {
            when (val result = open(server)) {
                is Opened -> attach(result.connection, server)
                is Refused -> finish(server, result.reason)
            }
        }
    }

    override fun disconnect() {
        endCurrent()
        _state.value = SessionState.Idle
    }

    /** Para tudo o que está em andamento e fecha a conexão, sem mexer no estado exibido. */
    private fun endCurrent() {
        val old = synchronized(lock) {
            val c = connection
            connection = null
            target = null
            c
        }
        job?.cancel()
        job = null
        old?.let { runCatching { it.disconnect() } }
    }

    private fun attach(conn: ControlConnection, server: PairedServer) {
        val slot = conn.welcome().slot.toInt()
        synchronized(lock) { connection = conn }
        _state.value = SessionState.Connected(server.serverId, server.name, slot)
        conn.start(
            object : SessionListener {
                override fun onConfig(config: ConfigInfo) {
                    val current = _state.value
                    if (current is SessionState.Connected && current.slot != config.slot.toInt()) {
                        _state.value = current.copy(slot = config.slot.toInt())
                    }
                }

                override fun onClosed(reason: String) = handleClosed(conn, server, reason)
            },
        )
    }

    private fun handleClosed(conn: ControlConnection, server: PairedServer, reason: String) {
        // Só reage se esta ainda é a conexão atual (um disconnect/connect novo já a trocou).
        val current = synchronized(lock) {
            if (connection !== conn) return
            connection = null
            true
        }
        if (!current) return
        Log.i(TAG, "sessão encerrada: $reason")
        when {
            reason == "local" -> _state.value = SessionState.Idle
            reason == "bye:kicked" -> finish(server, SessionEnd.Kicked)
            reason == "bye:revoked" -> finish(server, SessionEnd.Revoked)
            else -> reconnect(server)
        }
    }

    private fun reconnect(server: PairedServer) {
        job?.cancel()
        job = scope.launch {
            val started = System.currentTimeMillis()
            var attempt = 0
            while (isActive) {
                _state.value = SessionState.Reconnecting(server.name, attempt + 1)
                delay(Backoff[attempt.coerceAtMost(Backoff.lastIndex)])
                when (val result = open(server)) {
                    is Opened -> {
                        attach(result.connection, server)
                        return@launch
                    }
                    is Refused -> if (result.reason != SessionEnd.Lost) {
                        finish(server, result.reason)
                        return@launch
                    }
                }
                attempt++
                if (System.currentTimeMillis() - started > GIVE_UP_AFTER_MILLIS) {
                    finish(server, SessionEnd.Lost)
                    return@launch
                }
            }
        }
    }

    private fun finish(server: PairedServer, reason: SessionEnd) {
        synchronized(lock) { target = null }
        if (reason == SessionEnd.Revoked) onRevoked(server.serverId)
        _state.value = SessionState.Ended(server.name, reason)
    }

    private sealed interface Attempt
    private class Opened(val connection: ControlConnection) : Attempt
    private class Refused(val reason: SessionEnd) : Attempt

    /** Tenta cada endereço do PC; só desiste de um por falha de rede, não por recusa. */
    private fun open(server: PairedServer): Attempt {
        for (host in server.hosts) {
            try {
                return Opened(
                    ControlConnection.connect(
                        host,
                        server.tcpPort.toUShort(),
                        TrustPolicy.Pinned(server.fingerprintHex.hexToBytes()),
                        deviceInfo(),
                        Credential.DeviceToken(server.deviceTokenHex.hexToBytes()),
                    ),
                )
            } catch (e: CoreException.Network) {
                continue
            } catch (e: CoreException.Rejected) {
                return Refused(
                    when (e.reason) {
                        "revoked", "bad_auth" -> SessionEnd.Revoked
                        else -> SessionEnd.Lost
                    },
                )
            } catch (e: CoreException) {
                return Refused(SessionEnd.Lost)
            }
        }
        return Refused(SessionEnd.Lost)
    }
}

internal fun String.hexToBytes(): ByteArray = ByteArray(length / 2) { i ->
    substring(i * 2, i * 2 + 2).toInt(16).toByte()
}
