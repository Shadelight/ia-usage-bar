package com.shadelight.iausage.ui.screens

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import com.shadelight.iausage.data.AppPreferences
import com.shadelight.iausage.data.ProviderUsage
import com.shadelight.iausage.data.ThemeMode

@Composable
fun SettingsScreen(
    preferences: AppPreferences,
    availableProviders: List<ProviderUsage>,
    monitorEnabled: Boolean,
    onThemeChange: (ThemeMode) -> Unit,
    onUsedModeChange: (Boolean) -> Unit,
    onVisibleProviderIdsChange: (Set<String>?) -> Unit,
    onMonitorEnabledChange: (Boolean) -> Unit,
    onClearLocalData: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    var notificationsBlocked by remember { mutableStateOf(false) }
    val notificationPermission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        notificationsBlocked = !granted
        // Without the permission the service would run with nothing to show.
        onMonitorEnabledChange(granted)
    }
    val availableProviderIds = remember(availableProviders) {
        availableProviders.mapTo(linkedSetOf()) { it.id }
    }
    LazyColumn(
        modifier = modifier
            .fillMaxSize()
            .navigationBarsPadding()
            .testTag("settings-list"),
        contentPadding = PaddingValues(start = 16.dp, top = 16.dp, end = 16.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp),
    ) {
        item(key = "alerts") {
            SettingsSection("Avisos") {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        Text("Avisos en segundo plano")
                        Text(
                            "Reinicios, IAs por agotarse, mejor opción y PC desconectado. Mientras vigila, Android muestra una notificación fija.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    Switch(
                        checked = monitorEnabled,
                        onCheckedChange = { enable ->
                            val needsPermission = enable &&
                                Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
                                ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
                            if (needsPermission) {
                                notificationPermission.launch(Manifest.permission.POST_NOTIFICATIONS)
                            } else {
                                onMonitorEnabledChange(enable)
                            }
                        },
                    )
                }
                if (notificationsBlocked) {
                    Text(
                        "Android tiene bloqueadas las notificaciones de IA Usage. Actívalas en los ajustes del sistema.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
            }
        }

        item(key = "appearance") {
            SettingsSection("Apariencia") {
                ThemeMode.entries.forEach { mode ->
                    Row(
                        Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        RadioButton(selected = preferences.theme == mode, onClick = { onThemeChange(mode) })
                        Text(themeLabel(mode))
                    }
                }
            }
        }

        item(key = "data") {
            SettingsSection("Datos") {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        Text("Mostrar porcentaje restante")
                        Text("En vez de usado, en tarjetas y widget", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    Switch(checked = !preferences.usedMode, onCheckedChange = { onUsedModeChange(!it) })
                }
                TextButton(onClick = onClearLocalData) { Text("Limpiar datos locales", color = MaterialTheme.colorScheme.error) }
            }
        }

        if (availableProviders.isNotEmpty()) {
            item(key = "providers-header") {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Proveedores visibles", style = MaterialTheme.typography.titleMedium)
                    Text(
                        "Solo afecta lo que ves acá — no cambia los proveedores activos del PC.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            items(availableProviders, key = { it.id }) { provider ->
                val checked = preferences.visibleProviderIds?.contains(provider.id) ?: true
                ProviderVisibilityRow(
                    provider = provider,
                    checked = checked,
                    onCheckedChange = { isChecked ->
                        onVisibleProviderIdsChange(
                            updatedVisibleProviderIds(
                                current = preferences.visibleProviderIds,
                                availableProviderIds = availableProviderIds,
                                providerId = provider.id,
                                isChecked = isChecked,
                            ),
                        )
                    },
                )
            }

            item(key = "providers-divider") {
                HorizontalDivider(Modifier.padding(top = 4.dp))
            }
        }
    }
}

@Composable
private fun ProviderVisibilityRow(
    provider: ProviderUsage,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .heightIn(min = 48.dp)
            .toggleable(
                value = checked,
                role = Role.Checkbox,
                onValueChange = onCheckedChange,
            )
            .testTag("provider-visibility-${provider.id}")
            .padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        // The row owns the toggle action. Leaving this callback null keeps a tap
        // on the checkbox from firing both the child and parent handlers.
        Checkbox(checked = checked, onCheckedChange = null)
        Text(provider.name)
    }
}

internal fun updatedVisibleProviderIds(
    current: Set<String>?,
    availableProviderIds: Set<String>,
    providerId: String,
    isChecked: Boolean,
): Set<String>? {
    val currentIds = current ?: availableProviderIds
    val nextIds = if (isChecked) currentIds + providerId else currentIds - providerId
    return if (nextIds.size == availableProviderIds.size) null else nextIds
}

@Composable
private fun SettingsSection(title: String, content: @Composable () -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(title, style = MaterialTheme.typography.titleMedium)
        content()
        HorizontalDivider(Modifier.padding(top = 4.dp))
    }
}

private fun themeLabel(mode: ThemeMode) = when (mode) {
    ThemeMode.SYSTEM -> "Sistema"
    ThemeMode.LIGHT -> "Claro"
    ThemeMode.DARK -> "Oscuro"
}
