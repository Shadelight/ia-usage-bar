package com.shadelight.iausage.widget

import android.app.Activity
import android.appwidget.AppWidgetManager
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.appwidget.GlanceAppWidgetManager
import androidx.glance.appwidget.state.getAppWidgetState
import androidx.glance.appwidget.state.updateAppWidgetState
import androidx.glance.state.PreferencesGlanceStateDefinition
import androidx.lifecycle.lifecycleScope
import com.shadelight.iausage.data.Formatters
import com.shadelight.iausage.data.ProviderUsage
import com.shadelight.iausage.data.SecureStateStore
import com.shadelight.iausage.data.SyncPayload
import com.shadelight.iausage.data.providerShortName
import com.shadelight.iausage.data.providerVisual
import com.shadelight.iausage.ui.theme.IaUsageTheme
import kotlinx.coroutines.launch
import org.json.JSONObject

/** Widget configurator: live preview on top, grouped choices, and a save
 * button pinned to the bottom so it is always reachable (the old screen had
 * no scroll, so with several providers "Guardar" fell off-screen). Per
 * Android convention it starts as RESULT_CANCELED and only commits on save. */
class WidgetConfigurationActivity : ComponentActivity() {
    private var appWidgetId = AppWidgetManager.INVALID_APPWIDGET_ID

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setResult(Activity.RESULT_CANCELED)
        appWidgetId = intent?.extras?.getInt(AppWidgetManager.EXTRA_APPWIDGET_ID, AppWidgetManager.INVALID_APPWIDGET_ID)
            ?: AppWidgetManager.INVALID_APPWIDGET_ID
        if (appWidgetId == AppWidgetManager.INVALID_APPWIDGET_ID) {
            finish()
            return
        }

        val providers = runCatching {
            SecureStateStore(this).payload()?.let { SyncPayload.fromJson(JSONObject(it)) }
        }.getOrNull()?.snapshot?.providers?.let(::activeProviders).orEmpty()

        lifecycleScope.launch {
            // Reconfiguring an existing widget starts from its saved choices.
            val initial = runCatching {
                val glanceId = GlanceAppWidgetManager(this@WidgetConfigurationActivity).getGlanceIdBy(appWidgetId)
                readWidgetConfig(getAppWidgetState(this@WidgetConfigurationActivity, PreferencesGlanceStateDefinition, glanceId))
            }.getOrDefault(WidgetConfig())
            setContent {
                IaUsageTheme {
                    WidgetConfigurationScreen(providers, initial, onSave = ::save, onCancel = ::finish)
                }
            }
        }
    }

    private fun save(config: WidgetConfig) {
        lifecycleScope.launch {
            runCatching {
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
            }
            // Always let the launcher place the widget: a failed save degrades
            // to the defaults instead of trapping the user on this screen.
            setResult(Activity.RESULT_OK, Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, appWidgetId))
            finish()
        }
    }
}

private enum class WidgetKind { AUTO, SINGLE, COMPARE }

private fun WidgetConfig.kind() = when (mode) {
    WIDGET_MODE_AUTO -> WidgetKind.AUTO
    WIDGET_MODE_COMPARE -> WidgetKind.COMPARE
    else -> WidgetKind.SINGLE
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun WidgetConfigurationScreen(
    providers: List<ProviderUsage>,
    initial: WidgetConfig,
    onSave: (WidgetConfig) -> Unit,
    onCancel: () -> Unit,
) {
    var kind by remember { mutableStateOf(initial.kind()) }
    var singleId by remember { mutableStateOf(initial.mode.takeIf { initial.kind() == WidgetKind.SINGLE }) }
    var compareIds by remember { mutableStateOf(initial.visibleProviderIds) }
    var usedMode by remember { mutableStateOf(initial.usedMode) }
    var showBar by remember { mutableStateOf(initial.showBar) }
    var showReset by remember { mutableStateOf(initial.showReset) }
    var showUpdated by remember { mutableStateOf(initial.showStatus) }
    var error by remember { mutableStateOf<String?>(null) }

    val draft = WidgetConfig(
        mode = when (kind) {
            WidgetKind.AUTO -> WIDGET_MODE_AUTO
            WidgetKind.COMPARE -> WIDGET_MODE_COMPARE
            WidgetKind.SINGLE -> singleId ?: WIDGET_MODE_AUTO
        },
        usedMode = usedMode,
        showReset = showReset,
        showBar = showBar,
        showStatus = showUpdated,
        visibleProviderIds = if (kind == WidgetKind.COMPARE) compareIds else emptySet(),
    )

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Configurar widget") },
                navigationIcon = {
                    IconButton(onClick = onCancel) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Cancelar")
                    }
                },
            )
        },
        bottomBar = {
            Surface(tonalElevation = 3.dp) {
                Column(Modifier.fillMaxWidth().navigationBarsPadding().padding(16.dp)) {
                    error?.let {
                        Text(
                            text = it,
                            color = MaterialTheme.colorScheme.error,
                            style = MaterialTheme.typography.bodyMedium,
                            modifier = Modifier.padding(bottom = 8.dp),
                        )
                    }
                    Button(
                        onClick = {
                            val problem = widgetDraftError(kind == WidgetKind.SINGLE, singleId)
                            if (problem != null) error = problem else onSave(draft)
                        },
                        modifier = Modifier.fillMaxWidth().height(52.dp),
                    ) { Text("Guardar widget") }
                }
            }
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            WidgetPreview(providers, draft)

            ConfigSection("Qué muestra") {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    KindCard("Automático", "La IA con más disponible", kind == WidgetKind.AUTO, Modifier.weight(1f)) {
                        kind = WidgetKind.AUTO
                        error = null
                    }
                    KindCard("Una IA", "Siempre la misma", kind == WidgetKind.SINGLE, Modifier.weight(1f)) {
                        kind = WidgetKind.SINGLE
                        error = null
                    }
                    KindCard("Comparar", "Hasta $MAX_COMPARE_ROWS IAs", kind == WidgetKind.COMPARE, Modifier.weight(1f)) {
                        kind = WidgetKind.COMPARE
                        error = null
                    }
                }
                if (kind != WidgetKind.AUTO) {
                    if (providers.isEmpty()) {
                        Text(
                            "Vincula tu PC en la app para elegir tus IAs.",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    } else {
                        FlowRow(
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalArrangement = Arrangement.spacedBy(4.dp),
                        ) {
                            providers.forEach { provider ->
                                val selected = if (kind == WidgetKind.SINGLE) singleId == provider.id else provider.id in compareIds
                                FilterChip(
                                    selected = selected,
                                    onClick = {
                                        error = null
                                        if (kind == WidgetKind.SINGLE) {
                                            singleId = provider.id
                                        } else {
                                            compareIds = if (selected) compareIds - provider.id else compareIds + provider.id
                                        }
                                    },
                                    label = { Text(provider.name) },
                                )
                            }
                        }
                        if (kind == WidgetKind.COMPARE) {
                            Text(
                                "Sin selección se muestran las primeras $MAX_COMPARE_ROWS.",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }

            ConfigSection("Métrica") {
                SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                    SegmentedButton(
                        selected = usedMode,
                        onClick = { usedMode = true },
                        shape = SegmentedButtonDefaults.itemShape(index = 0, count = 2),
                    ) { Text("Usado") }
                    SegmentedButton(
                        selected = !usedMode,
                        onClick = { usedMode = false },
                        shape = SegmentedButtonDefaults.itemShape(index = 1, count = 2),
                    ) { Text("Disponible") }
                }
            }

            ConfigSection("Elementos visibles") {
                ToggleRow("Barra de progreso", showBar) { showBar = it }
                ToggleRow("Tiempo hasta el reinicio", showReset) { showReset = it }
                ToggleRow("Hora de actualización", showUpdated) { showUpdated = it }
            }
        }
    }
}

/** Compose rendition of the widget card with the same palette and the same
 * selection logic, so what you configure is what lands on the home screen. */
@Composable
private fun WidgetPreview(providers: List<ProviderUsage>, config: WidgetConfig) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("Vista previa", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Box(
            Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(24.dp))
                .background(WidgetPalette.Background)
                .padding(16.dp),
        ) {
            if (config.mode == WIDGET_MODE_COMPARE) {
                val (rows, hidden) = compareRows(providers, config.visibleProviderIds)
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text("Tus IAs", color = WidgetPalette.TextPrimary, fontWeight = FontWeight.Medium, modifier = Modifier.weight(1f))
                        if (hidden > 0) Text("+$hidden más", color = WidgetPalette.TextMuted, fontSize = 12.sp)
                    }
                    if (rows.isEmpty()) Text("Sin datos todavía", color = WidgetPalette.TextMuted)
                    rows.forEach { provider ->
                        val quota = Formatters.primaryQuota(provider, null)
                        Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                PreviewIcon(provider, 16)
                                Spacer(Modifier.width(8.dp))
                                Text(providerShortName(provider.id, provider.name), color = WidgetPalette.TextPrimary, modifier = Modifier.weight(1f))
                                Text(
                                    Formatters.percentText(quota?.usedPercent, config.usedMode),
                                    color = WidgetPalette.TextPrimary,
                                    fontWeight = FontWeight.Bold,
                                )
                            }
                            if (config.showBar) PreviewBar(quota?.usedPercent, height = 3)
                        }
                    }
                }
            } else {
                val picked = pickProvider(providers, config.mode)
                if (picked == null) {
                    Text("Sin datos todavía — vincula tu PC en la app.", color = WidgetPalette.TextMuted)
                } else {
                    val (provider, quota) = picked
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            PreviewIcon(provider, 20)
                            Spacer(Modifier.width(8.dp))
                            Text(
                                providerShortName(provider.id, provider.name),
                                color = WidgetPalette.TextPrimary,
                                fontWeight = FontWeight.Medium,
                                modifier = Modifier.weight(1f),
                            )
                            Box(Modifier.size(8.dp).background(WidgetPalette.status(usageLevel(quota.usedPercent)), CircleShape))
                        }
                        Row(verticalAlignment = Alignment.Bottom) {
                            Text(
                                Formatters.percentText(quota.usedPercent, config.usedMode),
                                color = WidgetPalette.TextPrimary,
                                fontSize = 36.sp,
                                fontWeight = FontWeight.Bold,
                            )
                            Spacer(Modifier.width(6.dp))
                            Text(metricWord(config), color = WidgetPalette.TextMuted, modifier = Modifier.padding(bottom = 6.dp))
                        }
                        if (config.showBar) PreviewBar(quota.usedPercent, height = 6)
                        val footer = widgetFooter(quota, config, null, stale = false)
                        if (footer.isNotEmpty()) Text(footer, color = WidgetPalette.TextMuted, fontSize = 12.sp)
                    }
                }
            }
        }
    }
}

@Composable
private fun PreviewIcon(provider: ProviderUsage, sizeDp: Int) {
    val visual = providerVisual(provider.id, provider.name)
    val icon = visual.icon
    if (icon != null) {
        Image(
            painterResource(icon),
            contentDescription = null,
            modifier = Modifier.size(sizeDp.dp),
            colorFilter = androidx.compose.ui.graphics.ColorFilter.tint(WidgetPalette.TextPrimary),
        )
    } else {
        Text(visual.shortCode, color = WidgetPalette.TextMuted, fontSize = 11.sp, fontWeight = FontWeight.Bold)
    }
}

@Composable
private fun PreviewBar(usedPercent: Double?, height: Int) {
    val fraction = ((usedPercent ?: 0.0) / 100.0).toFloat().coerceIn(0f, 1f)
    Box(
        Modifier
            .fillMaxWidth()
            .height(height.dp)
            .clip(RoundedCornerShape(50))
            .background(WidgetPalette.Track),
    ) {
        Box(
            Modifier
                .fillMaxWidth(fraction)
                .height(height.dp)
                .background(WidgetPalette.bar(usageLevel(usedPercent))),
        )
    }
}

@Composable
private fun ConfigSection(title: String, content: @Composable () -> Unit) {
    Card(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(title, style = MaterialTheme.typography.titleMedium)
            content()
        }
    }
}

@Composable
private fun KindCard(title: String, subtitle: String, selected: Boolean, modifier: Modifier, onClick: () -> Unit) {
    OutlinedCard(
        onClick = onClick,
        modifier = modifier.semantics { this.selected = selected },
        border = BorderStroke(
            if (selected) 2.dp else 1.dp,
            if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outline,
        ),
        colors = CardDefaults.outlinedCardColors(
            containerColor = if (selected) MaterialTheme.colorScheme.primary.copy(alpha = 0.12f) else MaterialTheme.colorScheme.surface,
        ),
    ) {
        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(title, style = MaterialTheme.typography.titleSmall)
            Text(subtitle, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@Composable
private fun ToggleRow(label: String, checked: Boolean, onCheckedChange: (Boolean) -> Unit) {
    Row(
        Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(label, modifier = Modifier.weight(1f))
        Switch(checked = checked, onCheckedChange = onCheckedChange)
    }
}
