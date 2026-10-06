package app.vynpad.discovery

import androidx.compose.runtime.Composable
import kotlinx.coroutines.flow.StateFlow

/** Por onde o PC foi encontrado (pode ser mais de um). */
enum class DiscoveryVia { Mdns, Broadcast, Manual }

/** Um PC com o VynPad, como a UI o enxerga. */
data class DiscoveredPc(
    val id: String,
    val name: String,
    val hosts: List<String>,
    val tcpPort: Int,
    val udpPort: Int,
    val os: String,
    val freeSlots: Int,
    val fingerprint: String,
    val via: Set<DiscoveryVia>,
) {
    val primaryHost: String get() = hosts.firstOrNull() ?: "—"
    val isManual: Boolean get() = DiscoveryVia.Manual in via
}

enum class ScanStatus { Idle, Scanning, Done }

/**
 * Motor de descoberta: mDNS e broadcast juntos, resultados mesclados por `id`.
 * A implementação é da plataforma (no Android, `NsdManager` + o núcleo Rust).
 */
interface DiscoveryEngine {
    val pcs: StateFlow<List<DiscoveredPc>>
    val status: StateFlow<ScanStatus>

    /** Começa uma busca nova (limpa a lista e anuncia este aparelho em "Próximos"). */
    fun start()

    /** Para a busca e o anúncio. */
    fun stop()

    /** Adiciona um PC digitado à mão; `false` se o endereço não for um IPv4:porta válido. */
    fun addManual(text: String): DiscoveredPc?
}

/** Engine ligado ao ciclo de vida da tela: é encerrado quando a composição sai. */
@Composable
expect fun rememberDiscoveryEngine(): DiscoveryEngine

/** Junta dois achados do mesmo PC (mesmo `id`): une endereços e origens, mantém o mais novo. */
fun DiscoveredPc.mergedWith(other: DiscoveredPc): DiscoveredPc = copy(
    name = other.name.ifBlank { name },
    hosts = (hosts + other.hosts).distinct().sorted(),
    tcpPort = if (other.tcpPort != 0) other.tcpPort else tcpPort,
    udpPort = if (other.udpPort != 0) other.udpPort else udpPort,
    os = other.os.ifBlank { os },
    freeSlots = other.freeSlots,
    fingerprint = other.fingerprint.ifBlank { fingerprint },
    via = via + other.via,
)

private val Ipv4 = Regex("""^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$""")

/** Lê `192.168.0.10` ou `192.168.0.10:47800`; `null` se o IPv4 ou a porta forem inválidos. */
fun parseHostPort(text: String, defaultPort: Int = DefaultControlPort): Pair<String, Int>? {
    val trimmed = text.trim()
    val host = trimmed.substringBefore(':')
    val port = if (':' in trimmed) trimmed.substringAfter(':').toIntOrNull() else defaultPort
    if (!Ipv4.matches(host) || port == null || port !in 1..65535) return null
    return host to port
}

const val DefaultControlPort = 47800
const val DefaultUdpPort = 47801

/** PC digitado à mão: só se conhecem o endereço e a porta. */
fun manualPc(host: String, port: Int) = DiscoveredPc(
    id = "manual:$host:$port",
    name = host,
    hosts = listOf(host),
    tcpPort = port,
    udpPort = DefaultUdpPort,
    os = "",
    freeSlots = 0,
    fingerprint = "",
    via = setOf(DiscoveryVia.Manual),
)
