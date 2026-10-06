package app.vynpad.pairing

import android.content.Context
import android.hardware.Sensor
import android.hardware.SensorManager
import android.os.Build
import android.os.Vibrator
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext
import app.vynpad.discovery.DeviceIdentity
import app.vynpad.discovery.DiscoveredPc
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import uniffi.vynpad_client_core.ControlConnection
import uniffi.vynpad_client_core.CoreException
import uniffi.vynpad_client_core.Credential
import uniffi.vynpad_client_core.DeviceInfo
import uniffi.vynpad_client_core.TrustPolicy
import uniffi.vynpad_client_core.parsePairingUri

internal class AndroidPairingController(context: Context) : PairingController {
    private val appContext = context.applicationContext
    private val identity = DeviceIdentity(appContext)
    private val store = SecureBlobStore(appContext, "paired_servers")

    private val _paired = MutableStateFlow(load())
    override val paired: StateFlow<List<PairedServer>> = _paired.asStateFlow()

    override val session = AndroidSessionManager(
        context = appContext,
        deviceInfo = ::deviceInfo,
        // O PC removeu este celular: o token não vale mais, então o pareamento some da lista.
        onRevoked = ::forget,
    )

    override suspend fun pairWithQr(uri: String): PairResult = withContext(Dispatchers.IO) {
        val info = try {
            parsePairingUri(uri)
        } catch (_: CoreException) {
            return@withContext PairResult.Failure(PairError.InvalidQr)
        }
        attempt(
            hosts = info.hosts,
            port = info.port.toInt(),
            trust = TrustPolicy.Pinned(info.fingerprint),
            credential = Credential.PairingToken(info.pairingToken),
            badAuth = PairError.ExpiredQr,
        )
    }

    override suspend fun pairWithPin(pc: DiscoveredPc, pin: String): PairResult = withContext(Dispatchers.IO) {
        // O fingerprint curto da descoberta pega um PC diferente do que o usuário escolheu.
        val trust = if (pc.fingerprint.length == SHORT_FP_CHARS) {
            TrustPolicy.ShortFingerprint(pc.fingerprint.lowercase())
        } else {
            TrustPolicy.FirstUse
        }
        attempt(
            hosts = pc.hosts,
            port = pc.tcpPort,
            trust = trust,
            credential = Credential.Pin(pin),
            badAuth = PairError.WrongPin,
        )
    }

    override fun forget(serverId: String) {
        val state = session.state.value
        if (state is SessionState.Connected && state.serverId == serverId) session.disconnect()
        save(_paired.value.filterNot { it.serverId == serverId })
    }

    /** Tenta cada endereço em ordem; só passa ao próximo quando o anterior não foi alcançado. */
    private fun attempt(
        hosts: List<String>,
        port: Int,
        trust: TrustPolicy,
        credential: Credential,
        badAuth: PairError,
    ): PairResult {
        if (hosts.isEmpty()) return PairResult.Failure(PairError.Unreachable)
        for (host in hosts) {
            try {
                val connection = ControlConnection.connect(host, port.toUShort(), trust, deviceInfo(), credential)
                val welcome = connection.welcome()
                val token = welcome.deviceToken
                if (token == null) {
                    connection.disconnect()
                    return PairResult.Failure(PairError.Unknown)
                }
                val server = PairedServer(
                    serverId = welcome.serverId,
                    name = welcome.serverName,
                    fingerprintHex = welcome.fingerprint.toHex(),
                    hosts = hosts,
                    tcpPort = port,
                    deviceTokenHex = token.toHex(),
                    slot = welcome.slot.toInt(),
                    pairedAtMillis = System.currentTimeMillis(),
                )
                save(_paired.value.filterNot { it.serverId == server.serverId } + server)
                // A conexão do pareamento vira a sessão: o PC continua vendo este celular conectado.
                session.adopt(connection, server)
                return PairResult.Success(server)
            } catch (e: CoreException.Network) {
                continue
            } catch (e: CoreException.Rejected) {
                return PairResult.Failure(
                    error = when (e.reason) {
                        "bad_auth" -> badAuth
                        "locked" -> PairError.Locked
                        "no_slot" -> PairError.NoSlot
                        "revoked" -> PairError.Revoked
                        "version" -> PairError.Version
                        else -> PairError.Unknown
                    },
                    retryAfterSeconds = e.retryAfterS?.toInt(),
                )
            } catch (e: CoreException.FingerprintMismatch) {
                return PairResult.Failure(PairError.FingerprintMismatch)
            } catch (e: CoreException) {
                return PairResult.Failure(PairError.Unknown)
            }
        }
        return PairResult.Failure(PairError.Unreachable)
    }

    private fun deviceInfo(): DeviceInfo {
        val sensors = appContext.getSystemService(Context.SENSOR_SERVICE) as? SensorManager
        val vibrator = appContext.getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator
        return DeviceInfo(
            deviceId = identity.id,
            name = identity.name,
            model = identity.model,
            osVer = Build.VERSION.RELEASE.take(32),
            appVer = APP_VERSION,
            gyro = sensors?.getDefaultSensor(Sensor.TYPE_GYROSCOPE) != null,
            rumble = vibrator?.hasVibrator() == true,
        )
    }

    private fun save(servers: List<PairedServer>) {
        _paired.value = servers
        val json = JSONArray().apply {
            servers.forEach { s ->
                put(
                    JSONObject()
                        .put("serverId", s.serverId)
                        .put("name", s.name)
                        .put("fingerprint", s.fingerprintHex)
                        .put("hosts", JSONArray(s.hosts))
                        .put("tcpPort", s.tcpPort)
                        .put("deviceToken", s.deviceTokenHex)
                        .put("slot", s.slot)
                        .put("pairedAt", s.pairedAtMillis),
                )
            }
        }
        store.write(json.toString())
    }

    private fun load(): List<PairedServer> {
        val text = store.read() ?: return emptyList()
        return runCatching {
            val array = JSONArray(text)
            (0 until array.length()).map { i ->
                val o = array.getJSONObject(i)
                val hosts = o.getJSONArray("hosts")
                PairedServer(
                    serverId = o.getString("serverId"),
                    name = o.getString("name"),
                    fingerprintHex = o.getString("fingerprint"),
                    hosts = (0 until hosts.length()).map { hosts.getString(it) },
                    tcpPort = o.getInt("tcpPort"),
                    deviceTokenHex = o.getString("deviceToken"),
                    slot = o.getInt("slot"),
                    pairedAtMillis = o.getLong("pairedAt"),
                )
            }
        }.getOrDefault(emptyList())
    }

    private companion object {
        const val SHORT_FP_CHARS = 16
        const val APP_VERSION = "0.1.0"
    }
}

private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }

@Composable
actual fun rememberPairingController(): PairingController {
    val context = LocalContext.current
    return remember(context) { AndroidPairingController(context) }
}
