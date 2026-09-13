package com.shadelight.iausage.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.shadelight.iausage.data.AppPreferences
import com.shadelight.iausage.data.ProviderUsage
import com.shadelight.iausage.data.ThemeMode

@Composable
fun SettingsScreen(
    preferences: AppPreferences,
    availableProviders: List<ProviderUsage>,
    onThemeChange: (ThemeMode) -> Unit,
    onUsedModeChange: (Boolean) -> Unit,
    onVisibleProviderIdsChange: (Set<String>?) -> Unit,
    onClearLocalData: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(20.dp)) {
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

        SettingsSection("Datos") {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text("Mostrar porcentaje disponible")
                    Text("En vez de usado, en tarjetas y widget", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Switch(checked = !preferences.usedMode, onCheckedChange = { onUsedModeChange(!it) })
            }
            TextButton(onClick = onClearLocalData) { Text("Limpiar datos locales", color = MaterialTheme.colorScheme.error) }
        }

        if (availableProviders.isNotEmpty()) {
            SettingsSection("Proveedores visibles") {
                Text(
                    "Solo afecta lo que ves acá — no cambia los proveedores activos del PC.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                availableProviders.forEach { provider ->
                    val checked = preferences.visibleProviderIds?.contains(provider.id) ?: true
                    Row(
                        Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Checkbox(
                            checked = checked,
                            onCheckedChange = { isChecked ->
                                val currentIds = preferences.visibleProviderIds ?: availableProviders.map { it.id }.toSet()
                                val nextIds = if (isChecked) currentIds + provider.id else currentIds - provider.id
                                onVisibleProviderIdsChange(if (nextIds.size == availableProviders.size) null else nextIds)
                            },
                        )
                        Text(provider.name)
                    }
                }
            }
        }
    }
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
