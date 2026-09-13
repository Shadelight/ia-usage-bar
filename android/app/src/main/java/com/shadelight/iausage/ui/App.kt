package com.shadelight.iausage.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.shadelight.iausage.data.ThemeMode
import com.shadelight.iausage.data.visibleProviders
import com.shadelight.iausage.ui.components.ErrorBanner
import com.shadelight.iausage.ui.screens.AboutScreen
import com.shadelight.iausage.ui.screens.DashboardScreen
import com.shadelight.iausage.ui.screens.DeviceScreen
import com.shadelight.iausage.ui.screens.NoPairingScreen
import com.shadelight.iausage.ui.screens.PendingPairingScreen
import com.shadelight.iausage.ui.screens.SettingsScreen

/** Screens behind pairing. Pairing itself is a special flow, not a
 * destination: it takes over whenever there is no linked PC yet. */
private enum class Screen { DASHBOARD, DEVICE, SETTINGS, ABOUT }

private fun titleFor(screen: Screen) = when (screen) {
    Screen.DASHBOARD -> "IA Usage"
    Screen.DEVICE -> "Dispositivo"
    Screen.SETTINGS -> "Ajustes"
    Screen.ABOUT -> "Acerca de"
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun UsageApp(viewModel: UsageViewModel, scanQr: () -> Unit) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val preferences by viewModel.preferences.collectAsStateWithLifecycle()
    var screen by remember { mutableStateOf(Screen.DASHBOARD) }
    var menuExpanded by remember { mutableStateOf(false) }

    val paired = state.payload != null

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(if (paired) titleFor(screen) else "IA Usage") },
                navigationIcon = {
                    if (paired && screen != Screen.DASHBOARD) {
                        IconButton(onClick = { screen = Screen.DASHBOARD }) {
                            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Volver")
                        }
                    }
                },
                actions = {
                    if (paired && screen == Screen.DASHBOARD) {
                        IconButton(onClick = { menuExpanded = true }) {
                            Icon(Icons.Filled.MoreVert, contentDescription = "Ajustes rápidos")
                        }
                        DropdownMenu(expanded = menuExpanded, onDismissRequest = { menuExpanded = false }) {
                            DropdownMenuItem(text = { Text("Proveedores visibles") }, onClick = { menuExpanded = false; screen = Screen.SETTINGS })
                            DropdownMenuItem(
                                text = { Text(if (preferences.usedMode) "Mostrar disponible" else "Mostrar usado") },
                                onClick = { menuExpanded = false; viewModel.setUsedMode(!preferences.usedMode) },
                            )
                            DropdownMenuItem(text = { Text("Actualizar ahora") }, onClick = { menuExpanded = false; viewModel.refresh() })
                            DropdownMenuItem(text = { Text("Dispositivo") }, onClick = { menuExpanded = false; screen = Screen.DEVICE })
                            DropdownMenuItem(text = { Text("Ajustes completos") }, onClick = { menuExpanded = false; screen = Screen.SETTINGS })
                            DropdownMenuItem(text = { Text("Acerca de") }, onClick = { menuExpanded = false; screen = Screen.ABOUT })
                        }
                    }
                },
            )
        },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding)) {
            state.error?.let { message ->
                ErrorBanner(message, onDismiss = viewModel::dismissError, modifier = Modifier.padding(horizontal = 16.dp))
            }
            when {
                state.payload != null -> {
                    val visible = visibleProviders(state.payload!!.snapshot.providers, preferences.visibleProviderIds)
                    when (screen) {
                        Screen.DASHBOARD -> DashboardScreen(
                            payload = state.payload!!,
                            providers = visible,
                            usedMode = preferences.usedMode,
                            refreshing = state.refreshing,
                            onRefresh = viewModel::refresh,
                        )
                        Screen.DEVICE -> state.pairing?.let { pairing ->
                            DeviceScreen(
                                pairing = pairing,
                                payload = state.payload,
                                refreshing = state.refreshing,
                                onRefreshNow = viewModel::refresh,
                                onDisconnect = viewModel::disconnect,
                            )
                        }
                        Screen.SETTINGS -> SettingsScreen(
                            preferences = preferences,
                            availableProviders = state.payload!!.snapshot.providers.filter { it.enabled },
                            onThemeChange = viewModel::setTheme,
                            onUsedModeChange = viewModel::setUsedMode,
                            onVisibleProviderIdsChange = viewModel::setVisibleProviderIds,
                            onClearLocalData = viewModel::clearLocalData,
                        )
                        Screen.ABOUT -> AboutScreen()
                    }
                }
                state.pairing == null -> NoPairingScreen(state.loading, scanQr, viewModel::setPairingUri)
                else -> PendingPairingScreen(state.pairing!!, state.loading, viewModel::pair, viewModel::clearPendingPair)
            }
        }
    }
}

/** Theme resolution shared by MainActivity's setContent call. */
@Composable
fun resolveDarkTheme(theme: ThemeMode): Boolean = when (theme) {
    ThemeMode.SYSTEM -> androidx.compose.foundation.isSystemInDarkTheme()
    ThemeMode.LIGHT -> false
    ThemeMode.DARK -> true
}
