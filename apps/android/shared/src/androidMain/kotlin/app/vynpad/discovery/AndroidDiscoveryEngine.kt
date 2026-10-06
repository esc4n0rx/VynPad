package app.vynpad.discovery

import android.content.Context
import android.net.wifi.WifiManager
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext
import java.net.Inet4Address
import java.net.NetworkInterface
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import uniffi.vynpad_client_core.BroadcastScanner
import uniffi.vynpad_client_core.DiscoveredServer
import uniffi.vynpad_client_core.ScanListener

/** Quanto dura uma busca (o PC costuma aparecer em menos de 3 s). */
private const val SCAN_MILLIS = 8_000L

/** Porta UDP informativa anunciada por este aparelho (o server nunca conecta nela). */
private const val ADVERTISED_UDP_PORT = 47803

internal class AndroidDiscoveryEngine(context: Context) : DiscoveryEngine {
    private val appContext = context.applicationContext
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val identity = DeviceIdentity(appContext)
    private val nsd = NsdSupport(appContext, scope, identity) { add(it) }

    private val byId = MutableStateFlow<Map<String, DiscoveredPc>>(emptyMap())
    private val _pcs = MutableStateFlow<List<DiscoveredPc>>(emptyList())
    override val pcs: StateFlow<List<DiscoveredPc>> = _pcs.asStateFlow()

    private val _status = MutableStateFlow(ScanStatus.Idle)
    override val status: StateFlow<ScanStatus> = _status.asStateFlow()

    private var scanJob: Job? = null
    private var scanner: BroadcastScanner? = null
    private var multicastLock: WifiManager.MulticastLock? = null

    override fun start() {
        stopScan()
        byId.value = emptyMap()
        _pcs.value = emptyList()
        _status.value = ScanStatus.Scanning

        acquireMulticastLock()
        nsd.startDiscovery()
        nsd.startAdvertising(ADVERTISED_UDP_PORT)

        val current = BroadcastScanner(broadcastAddresses())
        scanner = current
        val listener = object : ScanListener {
            override fun onServer(server: DiscoveredServer) = add(server.toPc())
        }
        scanJob = scope.launch(Dispatchers.IO) {
            // Rust bloqueia a thread até o fim da janela ou o cancelamento.
            val scan = launch { runCatching { current.scan(SCAN_MILLIS.toUInt(), listener) } }
            delay(SCAN_MILLIS)
            scan.cancel()
            nsd.stopDiscovery()
            _status.value = ScanStatus.Done
        }
    }

    override fun stop() {
        stopScan()
        nsd.stopAdvertising()
        releaseMulticastLock()
        if (_status.value == ScanStatus.Scanning) _status.value = ScanStatus.Idle
    }

    override fun addManual(text: String): DiscoveredPc? {
        val (host, port) = parseHostPort(text) ?: return null
        val pc = manualPc(host, port)
        add(pc)
        return byId.value[pc.id]
    }

    fun close() {
        stop()
        nsd.close()
        scope.cancel()
    }

    private fun stopScan() {
        scanner?.cancel()
        scanner = null
        scanJob?.cancel()
        scanJob = null
        nsd.stopDiscovery()
    }

    private fun add(pc: DiscoveredPc) {
        byId.update { map ->
            map + (pc.id to (map[pc.id]?.mergedWith(pc) ?: pc))
        }
        _pcs.value = byId.value.values.sortedWith(compareBy({ it.name.lowercase() }, { it.id }))
    }

    private fun acquireMulticastLock() {
        if (multicastLock?.isHeld == true) return
        val wifi = appContext.getSystemService(Context.WIFI_SERVICE) as? WifiManager ?: return
        multicastLock = wifi.createMulticastLock("vynpad-discovery").apply {
            setReferenceCounted(false)
            runCatching { acquire() }
        }
    }

    private fun releaseMulticastLock() {
        multicastLock?.takeIf { it.isHeld }?.release()
        multicastLock = null
    }

    /** Broadcast dirigido de cada interface IPv4 ativa (o limitado já é enviado pelo núcleo). */
    private fun broadcastAddresses(): List<String> = runCatching {
        NetworkInterface.getNetworkInterfaces().toList()
            .filter { it.isUp && !it.isLoopback }
            .flatMap { it.interfaceAddresses }
            .filter { it.address is Inet4Address }
            .mapNotNull { it.broadcast?.hostAddress }
            .distinct()
    }.getOrDefault(emptyList())
}

private fun DiscoveredServer.toPc() = DiscoveredPc(
    id = id,
    name = name,
    hosts = hosts,
    tcpPort = tcpPort.toInt(),
    udpPort = udpPort.toInt(),
    os = os,
    freeSlots = freeSlots.toInt(),
    fingerprint = shortFingerprint,
    via = setOf(DiscoveryVia.Broadcast),
)

@Composable
actual fun rememberDiscoveryEngine(): DiscoveryEngine {
    val context = LocalContext.current
    val engine = remember(context) { AndroidDiscoveryEngine(context) }
    DisposableEffect(engine) { onDispose { engine.close() } }
    return engine
}
