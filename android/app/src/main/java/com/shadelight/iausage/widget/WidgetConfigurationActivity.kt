package com.shadelight.iausage.widget

import android.app.Activity
import android.appwidget.AppWidgetManager
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.glance.appwidget.GlanceAppWidgetManager
import androidx.glance.appwidget.state.updateAppWidgetState
import androidx.lifecycle.lifecycleScope
import com.shadelight.iausage.data.SecureStateStore
import com.shadelight.iausage.data.SyncPayload
import com.shadelight.iausage.ui.theme.IaUsageTheme
import kotlinx.coroutines.launch
import org.json.JSONObject

/** Widget configuration screen (section 26): mode (automatic vs a specific
 * provider), used/available, and which rows to show. Per Android convention
 * it defaults to RESULT_CANCELED and only commits on RESULT_OK. */
class WidgetConfigurationActivity : ComponentActivity() {
    private var appWidgetId = AppWidgetManager.INVALID_APPWIDGET_ID

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setResult(Activity.RESULT_CANCELED)
        appWidgetId = intent?.extras?.getInt(AppWidgetManager.EXTRA_APPWIDGET_ID, AppWidgetManager.INVALID_APPWIDGET_ID)
            ?: AppWidgetManager.INVALID_APPWIDGET_ID
        if (appWidgetId == AppWidgetManager.INVALID_APPWIDGET_ID) {
            finish(); return
        }

        val providerNames = runCatching {
            SecureStateStore(this).payload()?.let { SyncPayload.fromJson(JSONObject(it)) }
        }.getOrNull()?.snapshot?.providers?.filter { it.enabled }.orEmpty().associate { it.id to it.name }

        setContent {
            IaUsageTheme {
                Surface(Modifier.fillMaxSize()) {
                    WidgetConfigurationScreen(providerNames) { config -> save(config) }
                }
            }
        }
    }

    private fun save(config: WidgetConfig) {
        lifecycleScope.launch {
            val glanceId = GlanceAppWidgetManager(this@WidgetConfigurationActivity).getGlanceIdBy(appWidgetId)
            updateAppWidgetState(this@WidgetConfigurationActivity, glanceId) { prefs ->
                prefs[WidgetPrefsKeys.MODE] = config.mode
                prefs[WidgetPrefsKeys.USED_MODE] = config.usedMode
                prefs[WidgetPrefsKeys.SHOW_RESET] = config.showReset
                prefs[WidgetPrefsKeys.SHOW_BAR] = config.showBar
                prefs[WidgetPrefsKeys.SHOW_STATUS] = config.showStatus
                prefs[WidgetPrefsKeys.VISIBLE_PROVIDERS] = config.visibleProviderIds
            }
            UsageWidget().update(this@WidgetConfigurationActivity, glanceId)
            val result = Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, appWidgetId)
            setResult(Activity.RESULT_OK, result)
            finish()
        }
    }
}

@Composable
private fun WidgetConfigurationScreen(providerNames: Map<String, String>, onSave: (WidgetConfig) -> Unit) {
    var mode by remember { mutableStateOf(WIDGET_MODE_AUTO) }
    var usedMode by remember { mutableStateOf(true) }
    var showReset by remember { mutableStateOf(true) }
    var showBar by remember { mutableStateOf(true) }
    var showStatus by remember { mutableStateOf(true) }

    Column(Modifier.fillMaxSize().padding(20.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text("Configurar widget", style = MaterialTheme.typography.headlineSmall)

        Text("Proveedor", style = MaterialTheme.typography.titleMedium)
        RadioRow("Automático (menor disponibilidad)", mode == WIDGET_MODE_AUTO) { mode = WIDGET_MODE_AUTO }
        providerNames.forEach { (id, name) -> RadioRow(name, mode == id) { mode = id } }

        Text("Porcentaje", style = MaterialTheme.typography.titleMedium)
        RadioRow("Usado", usedMode) { usedMode = true }
        RadioRow("Disponible", !usedMode) { usedMode = false }

        Text("Mostrar", style = MaterialTheme.typography.titleMedium)
        ToggleRow("Reset", showReset) { showReset = it }
        ToggleRow("Barra de progreso", showBar) { showBar = it }
        ToggleRow("Estado del PC", showStatus) { showStatus = it }

        Button(
            onClick = { onSave(WidgetConfig(mode, usedMode, showReset, showBar, showStatus)) },
            modifier = Modifier.fillMaxWidth(),
        ) { Text("Guardar") }
    }
}

@Composable
private fun RadioRow(label: String, selected: Boolean, onClick: () -> Unit) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        RadioButton(selected = selected, onClick = onClick)
        Text(label)
    }
}

@Composable
private fun ToggleRow(label: String, checked: Boolean, onCheckedChange: (Boolean) -> Unit) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
        Text(label)
        Switch(checked = checked, onCheckedChange = onCheckedChange)
    }
}
