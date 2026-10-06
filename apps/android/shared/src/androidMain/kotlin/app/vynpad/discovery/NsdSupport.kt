package app.vynpad.discovery

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.util.Log
import java.net.Inet4Address
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.launch

private const val TAG = "VynNsd"
private const val SERVER_TYPE = "_vynpad._tcp."
private const val CLIENT_TYPE = "_vynpad-client._tcp."

/**
 * mDNS pelo `NsdManager`: descobre `_vynpad._tcp` e anuncia `_vynpad-client._tcp` enquanto a
 * tela Conectar está aberta (assim o celular aparece em "Próximos" no server).
 */
@Suppress("DEPRECATION") // resolveService só foi substituído a partir do API 34; segue funcionando.
internal class NsdSupport(
    context: Context,
    private val scope: CoroutineScope,
    private val identity: DeviceIdentity,
    private val onFound: (DiscoveredPc) -> Unit,
) {
    private val nsd = context.getSystemService(Context.NSD_SERVICE) as NsdManager

    // O NsdManager antigo só resolve um serviço por vez: serializamos.
    private val resolveQueue = Channel<NsdServiceInfo>(Channel.UNLIMITED)
    private var discoveryListener: NsdManager.DiscoveryListener? = null
    private var registrationListener: NsdManager.RegistrationListener? = null

    init {
        scope.launch {
            for (info in resolveQueue) resolveOne(info)
        }
    }

    fun startDiscovery() {
        if (discoveryListener != null) return
        val listener = object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(serviceType: String) = Unit
            override fun onServiceFound(info: NsdServiceInfo) {
                resolveQueue.trySend(info)
            }
            override fun onServiceLost(info: NsdServiceInfo) = Unit
            override fun onDiscoveryStopped(serviceType: String) = Unit
            override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                Log.w(TAG, "mDNS indisponível ($errorCode); o broadcast cobre")
                discoveryListener = null
            }
            override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) = Unit
        }
        discoveryListener = listener
        runCatching { nsd.discoverServices(SERVER_TYPE, NsdManager.PROTOCOL_DNS_SD, listener) }
            .onFailure { discoveryListener = null }
    }

    fun stopDiscovery() {
        discoveryListener?.let { runCatching { nsd.stopServiceDiscovery(it) } }
        discoveryListener = null
    }

    fun startAdvertising(udpPort: Int) {
        if (registrationListener != null) return
        val info = NsdServiceInfo().apply {
            serviceName = identity.name
            serviceType = CLIENT_TYPE
            port = udpPort
            setAttribute("id", identity.id)
            setAttribute("name", identity.name)
            setAttribute("model", identity.model)
        }
        val listener = object : NsdManager.RegistrationListener {
            override fun onServiceRegistered(info: NsdServiceInfo) = Unit
            override fun onRegistrationFailed(info: NsdServiceInfo, errorCode: Int) {
                // O tipo `_vynpad-client` passa dos 15 caracteres da RFC 6763; alguns aparelhos recusam.
                Log.w(TAG, "anúncio recusado ($errorCode)")
                registrationListener = null
            }
            override fun onServiceUnregistered(info: NsdServiceInfo) = Unit
            override fun onUnregistrationFailed(info: NsdServiceInfo, errorCode: Int) = Unit
        }
        registrationListener = listener
        runCatching { nsd.registerService(info, NsdManager.PROTOCOL_DNS_SD, listener) }
            .onFailure { registrationListener = null }
    }

    fun stopAdvertising() {
        registrationListener?.let { runCatching { nsd.unregisterService(it) } }
        registrationListener = null
    }

    fun close() {
        stopDiscovery()
        stopAdvertising()
        resolveQueue.close()
    }

    private suspend fun resolveOne(info: NsdServiceInfo) {
        val done = kotlinx.coroutines.CompletableDeferred<Unit>()
        runCatching {
            nsd.resolveService(
                info,
                object : NsdManager.ResolveListener {
                    override fun onResolveFailed(info: NsdServiceInfo, errorCode: Int) {
                        done.complete(Unit)
                    }

                    override fun onServiceResolved(resolved: NsdServiceInfo) {
                        toPc(resolved)?.let(onFound)
                        done.complete(Unit)
                    }
                },
            )
        }.onFailure { done.complete(Unit) }
        // Sem resposta em 3 s, segue para o próximo da fila.
        kotlinx.coroutines.withTimeoutOrNull(3_000) { done.await() }
    }

    private fun toPc(resolved: NsdServiceInfo): DiscoveredPc? {
        fun txt(key: String): String? = resolved.attributes[key]?.toString(Charsets.UTF_8)
        val id = txt("id") ?: return null
        val host = (resolved.host as? Inet4Address)?.hostAddress ?: return null
        return DiscoveredPc(
            id = id,
            name = txt("name") ?: resolved.serviceName,
            hosts = listOf(host),
            tcpPort = resolved.port,
            udpPort = txt("udp")?.toIntOrNull() ?: 0,
            os = txt("os").orEmpty(),
            freeSlots = txt("free")?.toIntOrNull() ?: 0,
            fingerprint = txt("fp").orEmpty(),
            via = setOf(DiscoveryVia.Mdns),
        )
    }
}
