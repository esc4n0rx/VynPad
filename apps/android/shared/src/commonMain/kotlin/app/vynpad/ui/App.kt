package app.vynpad.ui

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.animateDpAsState
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
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import app.vynpad.discovery.DiscoveredPc
import app.vynpad.input.GamepadState
import app.vynpad.settings.AppSettings
import app.vynpad.skin.rememberSkinStore
import app.vynpad.settings.rememberSettingsStore
import kotlinx.coroutines.delay
import app.vynpad.discovery.rememberDiscoveryEngine
import app.vynpad.pairing.DeepLinks
import app.vynpad.pairing.SessionState
import app.vynpad.pairing.rememberPairingController
import app.vynpad.resources.Res
import app.vynpad.resources.nav_connect
import app.vynpad.resources.nav_settings
import app.vynpad.ui.components.GlassCard
import app.vynpad.ui.components.VynBackground
import app.vynpad.ui.components.VynIcon
import app.vynpad.ui.components.VynIconView
import app.vynpad.ui.components.VynText
import app.vynpad.ui.haptics.LocalHaptics
import app.vynpad.ui.haptics.ProvideHaptics
import app.vynpad.ui.screens.ConnectScreen
import app.vynpad.ui.screens.GamepadScreen
import app.vynpad.ui.screens.ManualPcSheet
import app.vynpad.ui.screens.PairingScreen
import app.vynpad.ui.screens.PcSheet
import app.vynpad.ui.screens.SettingsScreen
import app.vynpad.ui.screens.SplashScreen
import app.vynpad.ui.theme.PageTransitionMillis
import app.vynpad.ui.theme.PageTransitionOffsetDp
import app.vynpad.ui.theme.ThemeMode
import app.vynpad.ui.theme.VynTheme
import app.vynpad.ui.theme.vynSpring
import org.jetbrains.compose.resources.stringResource

enum class Screen { Connect, Settings, Gamepad }

private const val SAVE_DEBOUNCE_MILLIS = 400L

/** Raiz do app compartilhado (Android hoje, iOS depois). */
@Composable
fun VynApp() {
    var screen by remember { mutableStateOf(Screen.Connect) }
    // Os ajustes (tema, vibrações, layouts) vêm do aparelho e voltam a ele a cada mudança.
    val settingsStore = rememberSettingsStore()
    val skinStore = rememberSkinStore()
    var settings by remember { mutableStateOf(AppSettings.decode(settingsStore.load())) }
    LaunchedEffect(settings) {
        // Pequena espera: um arrastar de slider muda o valor várias vezes e só o último vai ao disco.
        delay(SAVE_DEBOUNCE_MILLIS)
        settingsStore.save(AppSettings.encode(settings))
        // Imagens de fundo que nenhum layout usa mais saem do aparelho.
        skinStore.gc(settings.layouts.mapNotNull { it.skin.image }.toSet())
    }
    val themeMode = settings.theme
    val hapticsEnabled = settings.hapticsEnabled
    val rumble = settings.rumble
    var splashDone by remember { mutableStateOf(false) }
    var selectedPc by remember { mutableStateOf<DiscoveredPc?>(null) }
    var manualOpen by remember { mutableStateOf(false) }

    var pairingOpen by remember { mutableStateOf(false) }
    var pairingPc by remember { mutableStateOf<DiscoveredPc?>(null) }
    var pairingUri by remember { mutableStateOf<String?>(null) }

    val engine = rememberDiscoveryEngine()
    val pairing = rememberPairingController()
    val pairedServers by pairing.paired.collectAsState()
    val pairedIds = remember(pairedServers) { pairedServers.map { it.serverId }.toSet() }
    val session by pairing.session.state.collectAsState()
    val padState = remember { GamepadState() }
    val rttMicros by pairing.session.rttMicros.collectAsState()
    val gyroActive by pairing.session.gyroActive.collectAsState()
    val gyroCalibrating by pairing.session.gyroCalibrating.collectAsState()
    // O controle na tela alimenta o envio ao PC assim que há uma sessão.
    LaunchedEffect(pairing, padState) { pairing.session.bindInput(padState) }
    LaunchedEffect(pairing, rumble) { pairing.session.setRumble(rumble) }
    val pcs by engine.pcs.collectAsState()
    val status by engine.status.collectAsState()

    // QR lido pela câmera nativa chega como link `vynpad://pair?...`.
    val deepLink by DeepLinks.pairingUri.collectAsState()
    LaunchedEffect(deepLink) {
        deepLink?.let {
            pairingUri = it
            pairingPc = null
            pairingOpen = true
            DeepLinks.pairingUri.value = null
        }
    }

    // A busca e o anúncio do celular só existem enquanto a tela Conectar está aberta.
    LaunchedEffect(screen, splashDone) {
        if (!splashDone) return@LaunchedEffect
        if (screen == Screen.Connect) engine.start() else engine.stop()
    }

    val offsetPx = with(LocalDensity.current) { PageTransitionOffsetDp.dp.roundToPx() }

    VynTheme(themeMode) {
        ProvideHaptics(hapticsEnabled, settings.hapticStrength) {
            PlatformBackHandler(enabled = pairingOpen) { pairingOpen = false }
            PlatformBackHandler(enabled = manualOpen) { manualOpen = false }
            PlatformBackHandler(enabled = selectedPc != null) { selectedPc = null }
            ImmersiveLandscapeEffect(active = screen == Screen.Gamepad)

            VynBackground {
                Crossfade(targetState = splashDone, animationSpec = tween(350), label = "splash") { done ->
                    if (!done) {
                        SplashScreen(onFinished = { splashDone = true })
                    } else {
                        AnimatedContent(
                            targetState = screen,
                            transitionSpec = {
                                (fadeIn(tween(PageTransitionMillis, delayMillis = 40)) +
                                    slideInVertically(tween(PageTransitionMillis)) { offsetPx })
                                    .togetherWith(fadeOut(tween(PageTransitionMillis / 2)))
                            },
                            label = "screen",
                        ) { target ->
                            when (target) {
                                Screen.Gamepad -> GamepadScreen(
                                    state = padState,
                                    session = session,
                                    rttMicros = rttMicros,
                                    gyroActive = gyroActive,
                                    gyroCalibrating = gyroCalibrating,
                                    onRecenterGyro = { pairing.session.recenterGyro() },
                                    settings = settings,
                                    onSettings = { settings = it },
                                    onBack = { screen = Screen.Connect },
                                    onDisconnect = { pairing.session.disconnect() },
                                    modifier = Modifier.windowInsetsPadding(WindowInsets.safeDrawing),
                                )
                                else -> Shell(tab = target, onTab = { screen = it }) {
                                    when (target) {
                                        Screen.Settings -> SettingsScreen(
                                            settings = settings,
                                            onSettings = { settings = it },
                                            onTestRumble = { pairing.session.testRumble() },
                                        )
                                        else -> ConnectScreen(
                                            pcs = pcs,
                                            status = status,
                                            pairedIds = pairedIds,
                                            session = session,
                                            rttMicros = rttMicros,
                                            onDisconnect = { pairing.session.disconnect() },
                                            onSearch = { engine.start() },
                                            onSelectPc = { selectedPc = it },
                                            onPairQr = {
                                                pairingPc = null
                                                pairingOpen = true
                                            },
                                            onAddByIp = { manualOpen = true },
                                            onPreviewGamepad = { screen = Screen.Gamepad },
                                        )
                                    }
                                }
                            }
                        }
                    }
                }

                if (pairingOpen) {
                    PairingScreen(
                        controller = pairing,
                        candidates = pcs,
                        preselected = pairingPc,
                        initialUri = pairingUri,
                        onClose = {
                            pairingOpen = false
                            pairingUri = null
                        },
                    )
                }

                if (splashDone && screen == Screen.Connect && !pairingOpen) {
                    PcSheet(
                        pc = selectedPc,
                        paired = selectedPc?.id in pairedIds,
                        connected = (session as? SessionState.Connected)?.serverId == selectedPc?.id,
                        onConnect = { pc ->
                            pairedServers.firstOrNull { it.serverId == pc.id }?.let(pairing.session::connect)
                            selectedPc = null
                        },
                        onDisconnect = {
                            pairing.session.disconnect()
                            selectedPc = null
                        },
                        onPair = {
                            pairingPc = it
                            selectedPc = null
                            pairingOpen = true
                        },
                        onForget = {
                            pairing.forget(it.id)
                            selectedPc = null
                        },
                        onDismiss = { selectedPc = null },
                    )
                    ManualPcSheet(
                        visible = manualOpen,
                        onDismiss = { manualOpen = false },
                        onAdd = { text ->
                            val added = engine.addManual(text)
                            if (added != null) {
                                manualOpen = false
                                selectedPc = added
                            }
                            added != null
                        },
                    )
                }
            }
        }
    }
}

/** Casca das telas principais: conteúdo rolável e a barra de abas flutuante, de vidro. */
@Composable
private fun Shell(tab: Screen, onTab: (Screen) -> Unit, content: @Composable () -> Unit) {
    Box(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing)) {
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(bottom = 108.dp)) {
            content()
        }
        TabBar(tab, onTab, Modifier.align(Alignment.BottomCenter).padding(bottom = 16.dp))
    }
}

private data class TabItem(val screen: Screen, val icon: VynIcon, val label: org.jetbrains.compose.resources.StringResource)

@Composable
private fun TabBar(selected: Screen, onSelect: (Screen) -> Unit, modifier: Modifier = Modifier) {
    val c = VynTheme.colors
    val haptics = LocalHaptics.current
    val items = listOf(
        TabItem(Screen.Connect, VynIcon.Connect, Res.string.nav_connect),
        TabItem(Screen.Settings, VynIcon.Settings, Res.string.nav_settings),
    )
    val index = items.indexOfFirst { it.screen == selected }.coerceAtLeast(0)
    GlassCard(modifier, radius = 32.dp, base = c.windowSolid.copy(alpha = 0.90f)) {
        BoxWithConstraints(Modifier.padding(6.dp)) {
            val itemWidth = 132.dp
            val x by animateDpAsState(itemWidth * index, vynSpring())
            Box(
                Modifier
                    .offset(x = x)
                    .width(itemWidth)
                    .height(52.dp)
                    .clip(CircleShape)
                    .background(c.accentSoft),
            )
            Row {
                items.forEach { item ->
                    val active = item.screen == selected
                    Row(
                        Modifier
                            .width(itemWidth)
                            .height(52.dp)
                            .clip(CircleShape)
                            .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {
                                if (!active) {
                                    haptics.tick()
                                    onSelect(item.screen)
                                }
                            },
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
                    ) {
                        val tint = if (active) c.accent else c.textSecondary
                        VynIconView(item.icon, tint = tint, size = 22.dp)
                        VynText(stringResource(item.label), style = VynTheme.typography.bodyStrong, color = tint)
                    }
                }
            }
        }
    }
}
