package app.vynpad.pairing

import android.content.Context
import android.net.wifi.WifiManager
import android.os.BatteryManager
import android.os.Build
import android.util.Log
import app.vynpad.input.GamepadSink
import app.vynpad.input.GamepadState
import app.vynpad.input.RumbleSettings
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
import uniffi.vynpad_client_core.InputPump
import uniffi.vynpad_client_core.PumpListener
import uniffi.vynpad_client_core.SessionListener
import uniffi.vynpad_client_core.TrustPolicy

private const val TAG = "VynSession"

/** Pausas entre tentativas de reconexão (PROTOCOLO §5.3): 0,5 s, 1 s, 2 s, 4 s e depois 5 s. */
private val Backoff = longArrayOf(500, 1_000, 2_000, 4_000, 5_000)

/** Desiste de reconectar depois deste tempo sem conseguir. */
private const val GIVE_UP_AFTER_MILLIS = 120_000L

/** De quanto em quanto tempo a bateria vai no pacote de input. */
private const val BATTERY_REFRESH_MILLIS = 30_000L

/** Peso mínimo de cada medição nova no RTT exibido (média móvel exponencial). */
private const val RTT_SMOOTHING = 0.2f

/** Nas primeiras medições a média é simples (peso 1/n): converge logo, sem arrastar a partida fria. */
private const val RTT_WARMUP_SAMPLES = 8

/**
 * Mantém a sessão com um PC pareado: o canal de controle (conecta com o `device_token`,
 * responde ao keepalive e reconecta sozinho se a rede cair) e o motor de envio do input por UDP,
 * ambos no núcleo Rust.
 */
internal class AndroidSessionManager(
    context: Context,
    private val deviceInfo: () -> DeviceInfo,
    /** Chamado quando o PC removeu este celular: o pareamento não vale mais. */
    private val onRevoked: (serverId: String) -> Unit,
) : SessionManager {
    private val appContext = context.applicationContext
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val _state = MutableStateFlow<SessionState>(SessionState.Idle)
    override val state: StateFlow<SessionState> = _state.asStateFlow()

    private val _rtt = MutableStateFlow<Int?>(null)
    private var rttSamples = 0
    override val rttMicros: StateFlow<Int?> = _rtt.asStateFlow()

    private val rumble = RumbleVibrator(appContext, scope)
    private var rumbleSettings = RumbleSettings()

    private val lock = Any()
    private var connection: ControlConnection? = null
    private var pump: InputPump? = null
    private var padState: GamepadState? = null
    private var target: PairedServer? = null
    private var job: Job? = null
    private var batteryJob: Job? = null
    private var wifiLock: WifiManager.WifiLock? = null

    override fun bindInput(state: GamepadState) {
        synchronized(lock) {
            padState = state
            // Se já há uma sessão, o controle passa a alimentá-la agora.
            pump?.let { connectSink(state, it) }
        }
    }

    override fun setRumble(settings: RumbleSettings) {
        rumbleSettings = settings
        rumble.configure(settings)
    }

    override fun testRumble() = rumble.test(rumbleSettings)

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
        stopInput()
        old?.let { runCatching { it.disconnect() } }
    }

    private fun attach(conn: ControlConnection, server: PairedServer) {
        val slot = conn.welcome().slot.toInt()
        synchronized(lock) { connection = conn }
        _state.value = SessionState.Connected(server.serverId, server.name, slot)
        startInput(conn, server)
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

    // ---- Motor de envio -------------------------------------------------------------------

    private fun startInput(conn: ControlConnection, server: PairedServer) {
        val engine = try {
            InputPump.start(
                conn.peerHost(),
                conn.welcome(),
                object : PumpListener {
                    override fun onRtt(rttUs: UInt) {
                        rumble.onAlive()
                        val measured = rttUs.toInt().coerceAtLeast(0)
                        val previous = _rtt.value
                        rttSamples = (rttSamples + 1).coerceAtMost(RTT_WARMUP_SAMPLES)
                        val weight = maxOf(RTT_SMOOTHING, 1f / rttSamples)
                        _rtt.value = if (previous == null) measured else (previous + (measured - previous) * weight).toInt()
                    }

                    override fun onRumble(rumbleId: UShort, large: UByte, small: UByte) =
                        rumble.onRumble(large.toInt(), small.toInt())

                    override fun onStopped(reason: String) {
                        // O contador de pacotes acabou ou o socket falhou: abre uma sessão nova.
                        if (reason != "local") handleClosed(conn, server, "pump:$reason")
                    }
                },
            )
        } catch (e: CoreException) {
            Log.w(TAG, "motor de envio não iniciou: $e")
            return
        }
        synchronized(lock) {
            pump = engine
            padState?.let { connectSink(it, engine) }
        }
        _rtt.value = null
        rttSamples = 0
        acquireWifiLock()
        batteryJob = scope.launch {
            while (isActive) {
                engine.setBattery(batteryPercent().toUByte())
                delay(BATTERY_REFRESH_MILLIS)
            }
        }
    }

    private fun stopInput() {
        val engine = synchronized(lock) {
            padState?.sink = null
            pump.also { pump = null }
        }
        batteryJob?.cancel()
        batteryJob = null
        rumble.reset()
        releaseWifiLock()
        _rtt.value = null
        engine?.let { runCatching { it.stop() } }
    }

    /** Liga o controle da tela ao motor: cada mudança vai direto, sem passar pelo Compose. */
    private fun connectSink(state: GamepadState, engine: InputPump) {
        state.sink = GamepadSink { buttons, lt, rt, lx, ly, rx, ry ->
            engine.setInput(
                buttons.toUShort(),
                lt.toUByte(),
                rt.toUByte(),
                lx.toShort(),
                ly.toShort(),
                rx.toShort(),
                ry.toShort(),
            )
        }
    }

    private fun batteryPercent(): Int {
        val manager = appContext.getSystemService(Context.BATTERY_SERVICE) as? BatteryManager ?: return UNKNOWN_BATTERY
        val level = manager.getIntProperty(BatteryManager.BATTERY_PROPERTY_CAPACITY)
        return if (level in 0..100) level else UNKNOWN_BATTERY
    }

    /** Wi-Fi em modo de baixa latência enquanto há sessão (evita o economizador atrasar os pacotes). */
    private fun acquireWifiLock() {
        if (wifiLock?.isHeld == true) return
        val wifi = appContext.getSystemService(Context.WIFI_SERVICE) as? WifiManager ?: return
        val mode = if (Build.VERSION.SDK_INT >= 29) {
            WifiManager.WIFI_MODE_FULL_LOW_LATENCY
        } else {
            @Suppress("DEPRECATION")
            WifiManager.WIFI_MODE_FULL_HIGH_PERF
        }
        wifiLock = wifi.createWifiLock(mode, "vynpad-session").apply {
            setReferenceCounted(false)
            runCatching { acquire() }
        }
    }

    private fun releaseWifiLock() {
        wifiLock?.takeIf { it.isHeld }?.release()
        wifiLock = null
    }

    // ---- Fim e reconexão ------------------------------------------------------------------

    private fun handleClosed(conn: ControlConnection, server: PairedServer, reason: String) {
        // Só reage se esta ainda é a conexão atual (um disconnect/connect novo já a trocou).
        synchronized(lock) {
            if (connection !== conn) return
            connection = null
        }
        Log.i(TAG, "sessão encerrada: $reason")
        stopInput()
        runCatching { conn.disconnect() }
        when (reason) {
            "local" -> _state.value = SessionState.Idle
            "bye:kicked" -> finish(server, SessionEnd.Kicked)
            "bye:revoked" -> finish(server, SessionEnd.Revoked)
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

    private companion object {
        const val UNKNOWN_BATTERY = 255
    }
}

internal fun String.hexToBytes(): ByteArray = ByteArray(length / 2) { i ->
    substring(i * 2, i * 2 + 2).toInt(16).toByte()
}
