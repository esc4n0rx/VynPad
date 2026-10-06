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
import app.vynpad.discovery.rememberDiscoveryEngine
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

/** Raiz do app compartilhado (Android hoje, iOS depois). */
@Composable
fun VynApp() {
    var screen by remember { mutableStateOf(Screen.Connect) }
    var themeMode by remember { mutableStateOf(ThemeMode.Dark) }
    var hapticsEnabled by remember { mutableStateOf(true) }
    var splashDone by remember { mutableStateOf(false) }
    var selectedPc by remember { mutableStateOf<DiscoveredPc?>(null) }
    var manualOpen by remember { mutableStateOf(false) }

    val engine = rememberDiscoveryEngine()
    val pcs by engine.pcs.collectAsState()
    val status by engine.status.collectAsState()

    // A busca e o anúncio do celular só existem enquanto a tela Conectar está aberta.
    LaunchedEffect(screen, splashDone) {
        if (!splashDone) return@LaunchedEffect
        if (screen == Screen.Connect) engine.start() else engine.stop()
    }

    val offsetPx = with(LocalDensity.current) { PageTransitionOffsetDp.dp.roundToPx() }

    VynTheme(themeMode) {
        ProvideHaptics(hapticsEnabled) {
            PlatformBackHandler(enabled = manualOpen) { manualOpen = false }
            PlatformBackHandler(enabled = selectedPc != null) { selectedPc = null }
            PlatformBackHandler(enabled = screen == Screen.Gamepad) { screen = Screen.Connect }
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
                                    onBack = { screen = Screen.Connect },
                                    modifier = Modifier.windowInsetsPadding(WindowInsets.safeDrawing),
                                )
                                else -> Shell(tab = target, onTab = { screen = it }) {
                                    when (target) {
                                        Screen.Settings -> SettingsScreen(
                                            themeMode = themeMode,
                                            onThemeMode = { themeMode = it },
                                            hapticsEnabled = hapticsEnabled,
                                            onHapticsEnabled = { hapticsEnabled = it },
                                        )
                                        else -> ConnectScreen(
                                            pcs = pcs,
                                            status = status,
                                            onSearch = { engine.start() },
                                            onSelectPc = { selectedPc = it },
                                            onAddByIp = { manualOpen = true },
                                            onPreviewGamepad = { screen = Screen.Gamepad },
                                        )
                                    }
                                }
                            }
                        }
                    }
                }

                if (splashDone && screen == Screen.Connect) {
                    PcSheet(pc = selectedPc, onDismiss = { selectedPc = null })
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
