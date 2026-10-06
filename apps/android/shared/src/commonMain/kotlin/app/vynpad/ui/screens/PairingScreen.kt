package app.vynpad.ui.screens

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import app.vynpad.discovery.DiscoveredPc
import app.vynpad.pairing.PairError
import app.vynpad.pairing.PairResult
import app.vynpad.pairing.PairedServer
import app.vynpad.pairing.PairingController
import app.vynpad.pairing.QrScanner
import app.vynpad.pairing.rememberOpenAppSettings
import app.vynpad.resources.Res
import app.vynpad.resources.pair_allow_camera
import app.vynpad.resources.pair_camera_denied_body
import app.vynpad.resources.pair_camera_denied_title
import app.vynpad.resources.pair_connecting
import app.vynpad.resources.pair_err_expired_qr
import app.vynpad.resources.pair_err_fingerprint
import app.vynpad.resources.pair_err_invalid_qr
import app.vynpad.resources.pair_err_locked
import app.vynpad.resources.pair_err_no_slot
import app.vynpad.resources.pair_err_revoked
import app.vynpad.resources.pair_err_unknown
import app.vynpad.resources.pair_err_unreachable
import app.vynpad.resources.pair_err_version
import app.vynpad.resources.pair_err_wrong_pin
import app.vynpad.resources.pair_method_pin
import app.vynpad.resources.pair_method_qr
import app.vynpad.resources.pair_open_settings
import app.vynpad.resources.pair_pin_check
import app.vynpad.resources.pair_pin_choose
import app.vynpad.resources.pair_pin_none
import app.vynpad.resources.pair_pin_title
import app.vynpad.resources.pair_qr_hint
import app.vynpad.resources.pair_retry
import app.vynpad.resources.pair_success_body
import app.vynpad.resources.pair_success_title
import app.vynpad.resources.pair_title
import app.vynpad.resources.pair_use_pin
import app.vynpad.resources.pair_use_qr
import app.vynpad.ui.components.BadgeTone
import app.vynpad.ui.components.ButtonKind
import app.vynpad.ui.components.GlassCard
import app.vynpad.ui.components.HintText
import app.vynpad.ui.components.NumericKeypad
import app.vynpad.ui.components.PinCells
import app.vynpad.ui.components.PulseRings
import app.vynpad.ui.components.ScanFrame
import app.vynpad.ui.components.SuccessBadge
import app.vynpad.ui.components.VynBadge
import app.vynpad.ui.components.VynButton
import app.vynpad.ui.components.VynIcon
import app.vynpad.ui.components.VynIconView
import app.vynpad.ui.components.VynSegmented
import app.vynpad.ui.components.VynText
import app.vynpad.ui.components.entrance
import app.vynpad.ui.haptics.LocalHaptics
import app.vynpad.ui.theme.VynTheme
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.jetbrains.compose.resources.stringResource

private enum class Method { Qr, Pin }

private sealed interface Phase {
    data object Idle : Phase
    data class Connecting(val label: String) : Phase
    data class Success(val server: PairedServer) : Phase
    data class Failed(val error: PairError, val retryAfterSeconds: Int?) : Phase
}

private const val PinLength = 6

/**
 * Pareamento em tela cheia: câmera para o QR ou PIN, com um seletor entre os dois. Fica sobre o
 * app (com fundo próprio) e fecha sozinho depois do selo de sucesso.
 */
@Composable
fun PairingScreen(
    controller: PairingController,
    candidates: List<DiscoveredPc>,
    preselected: DiscoveredPc?,
    initialUri: String?,
    onClose: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    val haptics = LocalHaptics.current
    val scope = rememberCoroutineScope()

    var method by remember { mutableStateOf(Method.Qr) }
    var phase by remember { mutableStateOf<Phase>(Phase.Idle) }
    var pin by remember { mutableStateOf("") }
    var errorTick by remember { mutableIntStateOf(0) }
    var scanKey by remember { mutableIntStateOf(0) }
    var cameraAttempt by remember { mutableIntStateOf(0) }
    var cameraGranted by remember { mutableStateOf<Boolean?>(null) }
    var chosen by remember { mutableStateOf(preselected ?: candidates.singleOrNull()) }

    fun fail(error: PairError, retryAfter: Int?) {
        haptics.reject()
        errorTick++
        phase = Phase.Failed(error, retryAfter)
        if (method == Method.Pin) pin = ""
    }

    fun handle(result: PairResult) {
        when (result) {
            is PairResult.Success -> {
                haptics.confirm()
                phase = Phase.Success(result.server)
            }
            is PairResult.Failure -> fail(result.error, result.retryAfterSeconds)
        }
    }

    fun submitPin(code: String) {
        val pc = chosen ?: return
        phase = Phase.Connecting(pc.name)
        scope.launch { handle(controller.pairWithPin(pc, code)) }
    }

    fun onQr(uri: String) {
        if (phase is Phase.Connecting || phase is Phase.Success) return
        phase = Phase.Connecting("PC")
        scope.launch { handle(controller.pairWithQr(uri)) }
    }

    // Link recebido de fora (QR da câmera nativa): já começa a parear.
    LaunchedEffect(initialUri) { initialUri?.let(::onQr) }

    // Depois do sucesso, deixa o selo respirar e fecha.
    LaunchedEffect(phase) {
        if (phase is Phase.Success) {
            delay(1900)
            onClose()
        }
    }

    Box(
        modifier
            .fillMaxSize()
            .background(c.windowSolid)
            .windowInsetsPadding(WindowInsets.safeDrawing),
    ) {
        Column(
            Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 20.dp),
            verticalArrangement = Arrangement.spacedBy(18.dp),
        ) {
            Row(
                Modifier.fillMaxWidth().padding(top = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Box(
                    Modifier
                        .size(44.dp)
                        .background(c.control, CircleShape)
                        .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {
                            haptics.tick()
                            onClose()
                        },
                    contentAlignment = Alignment.Center,
                ) { VynIconView(VynIcon.Close, tint = c.textPrimary, size = 22.dp) }
                VynText(stringResource(Res.string.pair_title), style = t.title)
            }

            VynSegmented(
                options = listOf(
                    Method.Qr to stringResource(Res.string.pair_method_qr),
                    Method.Pin to stringResource(Res.string.pair_method_pin),
                ),
                selected = method,
                onSelect = {
                    method = it
                    pin = ""
                    phase = Phase.Idle
                },
                modifier = Modifier.entrance(0),
            )

            AnimatedContent(
                targetState = method,
                transitionSpec = {
                    (fadeIn(tween(180, delayMillis = 40)) + slideInVertically(tween(180)) { 24 })
                        .togetherWith(fadeOut(tween(100)))
                },
                label = "pair-method",
            ) { current ->
                when (current) {
                    Method.Qr -> QrPane(
                        phase = phase,
                        scanKey = scanKey,
                        cameraAttempt = cameraAttempt,
                        cameraGranted = cameraGranted,
                        onCode = ::onQr,
                        onPermission = { cameraGranted = it },
                        onAllowCamera = { cameraAttempt++ },
                        onRetry = {
                            phase = Phase.Idle
                            scanKey++
                        },
                        onUsePin = {
                            method = Method.Pin
                            phase = Phase.Idle
                        },
                    )
                    Method.Pin -> PinPane(
                        phase = phase,
                        pin = pin,
                        errorTick = errorTick,
                        candidates = candidates,
                        chosen = chosen,
                        onChoose = {
                            chosen = it
                            phase = Phase.Idle
                        },
                        onDigit = { d ->
                            if (phase is Phase.Connecting || phase is Phase.Success || pin.length >= PinLength) return@PinPane
                            if (phase is Phase.Failed) phase = Phase.Idle
                            pin += d
                            if (pin.length == PinLength) submitPin(pin)
                        },
                        onDelete = {
                            if (phase is Phase.Connecting) return@PinPane
                            if (phase is Phase.Failed) phase = Phase.Idle
                            pin = pin.dropLast(1)
                        },
                        onUseQr = {
                            method = Method.Qr
                            phase = Phase.Idle
                        },
                    )
                }
            }
            Spacer(Modifier.height(24.dp))
        }

        // Conectando e sucesso cobrem a tela inteira.
        when (val current = phase) {
            is Phase.Connecting -> Overlay {
                PulseRings(Modifier.size(150.dp))
                VynText(stringResource(Res.string.pair_connecting, current.label), style = t.subtitle)
            }
            is Phase.Success -> Overlay {
                SuccessBadge(Modifier.size(160.dp))
                VynText(stringResource(Res.string.pair_success_title), style = t.title)
                VynText(
                    stringResource(Res.string.pair_success_body, current.server.name, current.server.slot + 1),
                    style = t.body,
                    color = c.textSecondary,
                )
            }
            else -> Unit
        }
    }
}

@Composable
private fun Overlay(content: @Composable () -> Unit) {
    val c = VynTheme.colors
    Box(
        Modifier
            .fillMaxSize()
            .background(c.windowSolid.copy(alpha = 0.94f))
            // Engole os toques enquanto a operação roda.
            .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {},
        contentAlignment = Alignment.Center,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(18.dp)) {
            content()
        }
    }
}

@Composable
private fun QrPane(
    phase: Phase,
    scanKey: Int,
    cameraAttempt: Int,
    cameraGranted: Boolean?,
    onCode: (String) -> Unit,
    onPermission: (Boolean) -> Unit,
    onAllowCamera: () -> Unit,
    onRetry: () -> Unit,
    onUsePin: () -> Unit,
) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    val openSettings = rememberOpenAppSettings()
    val failed = phase as? Phase.Failed
    Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
        GlassCard(Modifier.fillMaxWidth().aspectRatio(1f), radius = 28.dp) {
            if (cameraGranted == false) {
                Column(
                    Modifier.fillMaxSize().padding(24.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp, Alignment.CenterVertically),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    VynText(stringResource(Res.string.pair_camera_denied_title), style = t.subtitle)
                    HintText(stringResource(Res.string.pair_camera_denied_body), c.textSecondary)
                    Spacer(Modifier.height(4.dp))
                    VynButton(stringResource(Res.string.pair_allow_camera), onAllowCamera, modifier = Modifier.fillMaxWidth())
                    VynButton(stringResource(Res.string.pair_open_settings), openSettings, kind = ButtonKind.Secondary, modifier = Modifier.fillMaxWidth())
                }
            } else {
                androidx.compose.runtime.key(scanKey) {
                    Box(Modifier.fillMaxSize().clip(RoundedCornerShape(28.dp))) {
                        QrScanner(
                            onCode = onCode,
                            onPermission = onPermission,
                            attempt = cameraAttempt,
                            modifier = Modifier.fillMaxSize(),
                        )
                        ScanFrame(tint = if (failed != null) c.danger else c.accent)
                    }
                }
            }
        }
        if (failed != null) {
            HintText(errorText(failed), c.danger)
            VynButton(stringResource(Res.string.pair_retry), onRetry, modifier = Modifier.fillMaxWidth())
        } else {
            HintText(stringResource(Res.string.pair_qr_hint), c.textSecondary)
        }
        VynButton(stringResource(Res.string.pair_use_pin), onUsePin, kind = ButtonKind.Ghost, modifier = Modifier.fillMaxWidth())
    }
}

@Composable
private fun PinPane(
    phase: Phase,
    pin: String,
    errorTick: Int,
    candidates: List<DiscoveredPc>,
    chosen: DiscoveredPc?,
    onChoose: (DiscoveredPc) -> Unit,
    onDigit: (Char) -> Unit,
    onDelete: () -> Unit,
    onUseQr: () -> Unit,
) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    val failed = phase as? Phase.Failed
    Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(16.dp)) {
        if (chosen == null) {
            VynText(stringResource(Res.string.pair_pin_choose), style = t.subtitle)
            if (candidates.isEmpty()) {
                HintText(stringResource(Res.string.pair_pin_none), c.textSecondary)
            } else {
                candidates.forEach { pc -> PcChoice(pc, selected = false, onClick = { onChoose(pc) }) }
            }
            VynButton(stringResource(Res.string.pair_use_qr), onUseQr, kind = ButtonKind.Ghost, modifier = Modifier.fillMaxWidth())
        } else {
            if (candidates.size > 1) {
                candidates.forEach { pc -> PcChoice(pc, selected = pc.id == chosen.id, onClick = { onChoose(pc) }) }
            } else {
                PcChoice(chosen, selected = true, onClick = {})
            }
            if (chosen.fingerprint.isNotBlank()) {
                VynBadge(
                    stringResource(Res.string.pair_pin_check, chosen.fingerprint.chunked(4).joinToString(" ")),
                    BadgeTone.Info,
                )
            }
            VynText(stringResource(Res.string.pair_pin_title), style = t.subtitle, modifier = Modifier.padding(top = 4.dp))
            PinCells(pin = pin, length = PinLength, errorTick = errorTick)
            if (failed != null) {
                HintText(errorText(failed), c.danger)
            } else {
                Spacer(Modifier.height(20.dp))
            }
            NumericKeypad(
                onDigit = onDigit,
                onDelete = onDelete,
                enabled = !(failed?.error == PairError.Locked && (failed.retryAfterSeconds ?: 0) > 0),
            )
            VynButton(stringResource(Res.string.pair_use_qr), onUseQr, kind = ButtonKind.Ghost, modifier = Modifier.fillMaxWidth())
        }
    }
}

@Composable
private fun PcChoice(pc: DiscoveredPc, selected: Boolean, onClick: () -> Unit) {
    val c = VynTheme.colors
    val t = VynTheme.typography
    val haptics = LocalHaptics.current
    GlassCard(
        Modifier
            .fillMaxWidth()
            .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {
                haptics.tick()
                onClick()
            },
        radius = 18.dp,
        glow = if (selected) c.accent else null,
    ) {
        Row(
            Modifier.fillMaxWidth().padding(14.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            VynIconView(VynIcon.Monitor, tint = if (selected) c.accent else c.textSecondary, size = 24.dp)
            Column(Modifier.weight(1f)) {
                VynText(pc.name, style = t.bodyStrong, maxLines = 1)
                VynText(pc.primaryHost, style = t.caption, color = c.textSecondary)
            }
        }
    }
}

@Composable
private fun errorText(failed: Phase.Failed): String = when (failed.error) {
    PairError.WrongPin -> stringResource(Res.string.pair_err_wrong_pin)
    PairError.ExpiredQr -> stringResource(Res.string.pair_err_expired_qr)
    PairError.Locked -> stringResource(Res.string.pair_err_locked, failed.retryAfterSeconds ?: 60)
    PairError.NoSlot -> stringResource(Res.string.pair_err_no_slot)
    PairError.Revoked -> stringResource(Res.string.pair_err_revoked)
    PairError.Version -> stringResource(Res.string.pair_err_version)
    PairError.Unreachable -> stringResource(Res.string.pair_err_unreachable)
    PairError.FingerprintMismatch -> stringResource(Res.string.pair_err_fingerprint)
    PairError.InvalidQr -> stringResource(Res.string.pair_err_invalid_qr)
    PairError.Unknown -> stringResource(Res.string.pair_err_unknown)
}
