package app.vynpad.pairing

import androidx.compose.runtime.Composable
import app.vynpad.discovery.DiscoveredPc
import kotlinx.coroutines.flow.StateFlow

/** Um PC pareado: o necessário para reconectar sem pedir PIN, com o certificado fixado. */
data class PairedServer(
    val serverId: String,
    val name: String,
    /** SHA-256 do certificado (hex minúsculo, 64 caracteres): o pinning do TLS. */
    val fingerprintHex: String,
    val hosts: List<String>,
    val tcpPort: Int,
    /** Token do dispositivo (hex). Segredo: só existe cifrado em disco. */
    val deviceTokenHex: String,
    val slot: Int,
    val pairedAtMillis: Long,
)

/** Por que o pareamento não aconteceu. A UI traduz para uma mensagem e uma ação. */
enum class PairError {
    /** O PIN digitado não confere. */
    WrongPin,

    /** O QR já foi usado ou expirou (vale 5 min, uso único). */
    ExpiredQr,

    /** Excesso de PINs errados; tente de novo depois de `retryAfterSeconds`. */
    Locked,

    /** Os 4 slots do PC estão ocupados. */
    NoSlot,

    /** Este aparelho foi revogado no PC. */
    Revoked,

    /** Versões do app e do PC não conversam. */
    Version,

    /** Não achou o PC na rede (desligado, outra rede, firewall). */
    Unreachable,

    /** O certificado não é o do PC esperado. */
    FingerprintMismatch,

    /** O QR lido não é um QR do VynPad. */
    InvalidQr,

    /** Qualquer outro problema. */
    Unknown,
}

sealed interface PairResult {
    data class Success(val server: PairedServer) : PairResult

    data class Failure(val error: PairError, val retryAfterSeconds: Int? = null) : PairResult
}

/** Pareamento e a lista de PCs pareados. A implementação é da plataforma. */
interface PairingController {
    val paired: StateFlow<List<PairedServer>>

    /** A conexão com o PC; o pareamento bem-sucedido já a deixa conectada. */
    val session: SessionManager

    /** Pareia pela URI `vynpad://pair?...` lida do QR. */
    suspend fun pairWithQr(uri: String): PairResult

    /** Pareia com o PIN de 6 dígitos mostrado no PC. */
    suspend fun pairWithPin(pc: DiscoveredPc, pin: String): PairResult

    /** Esquece o PC (o PC continua listando este aparelho até ele ser revogado lá). */
    fun forget(serverId: String)
}

@Composable
expect fun rememberPairingController(): PairingController
