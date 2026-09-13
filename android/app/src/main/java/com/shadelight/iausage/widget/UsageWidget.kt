package com.shadelight.iausage.widget

import android.content.Context
import android.content.Intent
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.Image
import androidx.glance.ImageProvider
import androidx.glance.LocalSize
import androidx.glance.action.clickable
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.LinearProgressIndicator
import androidx.glance.appwidget.SizeMode
import androidx.glance.appwidget.action.actionRunCallback
import androidx.glance.appwidget.action.actionStartActivity
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.provideContent
import androidx.glance.background
import androidx.glance.layout.Column
import androidx.glance.layout.Row
import androidx.glance.layout.Spacer
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.fillMaxWidth
import androidx.glance.layout.height
import androidx.glance.layout.padding
import androidx.glance.layout.size
import androidx.glance.layout.width
import androidx.glance.state.PreferencesGlanceStateDefinition
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.text.TextStyle
import androidx.glance.unit.ColorProvider
import com.shadelight.iausage.MainActivity
import com.shadelight.iausage.R
import com.shadelight.iausage.data.Formatters
import com.shadelight.iausage.data.ProviderUsage
import com.shadelight.iausage.data.SecureStateStore
import com.shadelight.iausage.data.SyncPayload
import com.shadelight.iausage.data.UsageQuota
import com.shadelight.iausage.data.UsageSyncRepository
import com.shadelight.iausage.data.providerVisual
import org.json.JSONObject

private val WidgetBackground = Color(0xFF15233B)
private val WidgetAccent = Color(0xFFFF7043)
private val WidgetText = Color.White
private val WidgetMuted = Color(0xFFC5D3E6)
private val WidgetWarn = Color(0xFFFBBF24)

private val SmallSize = DpSize(110.dp, 110.dp)
private val MediumSize = DpSize(250.dp, 110.dp)
private val LargeSize = DpSize(250.dp, 250.dp)

class UsageWidget : GlanceAppWidget() {
    override val stateDefinition = PreferencesGlanceStateDefinition
    override val sizeMode = SizeMode.Responsive(setOf(SmallSize, MediumSize, LargeSize))

    override suspend fun provideGlance(context: Context, id: GlanceId) {
        // A widget host (notably Samsung One UI) shows a permanent "can't
        // load widget" placeholder if provideGlance ever throws, so any
        // failure here must degrade to an error row instead of crashing.
        val payload = runCatching {
            SecureStateStore(context).payload()?.let { runCatching { SyncPayload.fromJson(JSONObject(it)) }.getOrNull() }
        }.getOrNull()
        provideContent { WidgetContent(payload) }
    }

    @Composable
    private fun WidgetContent(payload: SyncPayload?) {
        val size = LocalSize.current
        val prefs = androidx.glance.currentState<androidx.datastore.preferences.core.Preferences>()
        val config = readWidgetConfig(prefs)
        val context = androidx.glance.LocalContext.current

        Column(
            modifier = GlanceModifier
                .fillMaxSize()
                .background(WidgetBackground)
                .cornerRadius(16.dp)
                .padding(12.dp)
                .clickable(actionStartActivity(Intent(context, MainActivity::class.java))),
        ) {
            Row {
                Image(ImageProvider(R.drawable.ic_ia_usage_mark), contentDescription = null, modifier = GlanceModifier.size(16.dp))
                Spacer(GlanceModifier.width(6.dp))
                Text("IA Usage", style = TextStyle(color = ColorProvider(WidgetAccent), fontWeight = FontWeight.Bold))
                Spacer(GlanceModifier.defaultWeight())
                Image(
                    ImageProvider(android.R.drawable.stat_notify_sync),
                    contentDescription = "Actualizar",
                    modifier = GlanceModifier.size(14.dp).clickable(actionRunCallback<RefreshWidgetAction>()),
                )
            }
            Spacer(GlanceModifier.height(6.dp))

            val providers = payload?.snapshot?.providers?.filter { it.enabled } ?: emptyList()
            when {
                payload == null -> Text("Sin datos todavía\nAbre la app para vincular tu PC", style = TextStyle(color = ColorProvider(WidgetMuted)))
                providers.isEmpty() -> Text("Sin proveedores activos", style = TextStyle(color = ColorProvider(WidgetMuted)))
                size.width < MediumSize.width -> SmallContent(providers, config)
                size.height < LargeSize.height -> MediumContent(providers, config)
                else -> LargeContent(providers, config)
            }

            val stale = payload != null && Formatters.isSnapshotStale(providers)
            if (config.showStatus && payload != null) {
                Spacer(GlanceModifier.height(4.dp))
                Text(
                    if (stale) "⚠ Datos de hace ${ageOnly(payload.generatedAt)}" else "Actualizado ${Formatters.formatAge(payload.generatedAt)}",
                    style = TextStyle(color = ColorProvider(if (stale) WidgetWarn else WidgetMuted)),
                )
            }
        }
    }

    private fun ageOnly(generatedAt: String) = Formatters.formatAge(generatedAt).removePrefix("hace ")

    @Composable
    private fun SmallContent(providers: List<ProviderUsage>, config: WidgetConfig) {
        val (provider, quota) = selectProvider(providers, config) ?: return
        val visual = providerVisual(provider.id, provider.name)
        Column {
            Row {
                visual.icon?.let { Image(ImageProvider(it), contentDescription = null, modifier = GlanceModifier.size(14.dp)) }
                Spacer(GlanceModifier.width(4.dp))
                Text(provider.name, style = TextStyle(color = ColorProvider(WidgetText), fontWeight = FontWeight.Medium))
            }
            Text(
                Formatters.percentText(quota.usedPercent, config.usedMode) + (if (config.usedMode) " usado" else " libre"),
                style = TextStyle(color = ColorProvider(WidgetText)),
            )
            if (config.showBar) ProgressRow(quota.usedPercent)
            if (config.showReset) Formatters.formatResetIn(quota.resetInSeconds)?.let {
                Text(it, style = TextStyle(color = ColorProvider(WidgetMuted)))
            }
        }
    }

    @Composable
    private fun MediumContent(providers: List<ProviderUsage>, config: WidgetConfig) {
        val visible = (if (config.visibleProviderIds.isEmpty()) providers else providers.filter { it.id in config.visibleProviderIds }).take(3)
        Column {
            visible.forEach { provider ->
                val quota = Formatters.primaryQuota(provider, null)
                Row {
                    Text(provider.name, style = TextStyle(color = ColorProvider(WidgetText)), modifier = GlanceModifier.defaultWeight())
                    Text(Formatters.percentText(quota?.usedPercent, config.usedMode), style = TextStyle(color = ColorProvider(WidgetText)))
                }
                if (config.showBar) ProgressRow(quota?.usedPercent)
            }
        }
    }

    @Composable
    private fun LargeContent(providers: List<ProviderUsage>, config: WidgetConfig) {
        val visible = if (config.visibleProviderIds.isEmpty()) providers else providers.filter { it.id in config.visibleProviderIds }
        Column {
            visible.forEach { provider ->
                Text(provider.name, style = TextStyle(color = ColorProvider(WidgetAccent), fontWeight = FontWeight.Medium))
                provider.quotas.take(2).forEach { quota ->
                    Row {
                        Text(quota.label, style = TextStyle(color = ColorProvider(WidgetText)), modifier = GlanceModifier.defaultWeight())
                        Text(Formatters.percentText(quota.usedPercent, config.usedMode), style = TextStyle(color = ColorProvider(WidgetText)))
                    }
                    if (config.showBar) ProgressRow(quota.usedPercent)
                }
                Spacer(GlanceModifier.height(4.dp))
            }
        }
    }

    @Composable
    private fun ProgressRow(usedPercent: Double?) {
        LinearProgressIndicator(
            modifier = GlanceModifier.fillMaxWidth().padding(vertical = 2.dp),
            progress = ((usedPercent ?: 0.0) / 100.0).toFloat().coerceIn(0f, 1f),
            color = ColorProvider(WidgetAccent),
            backgroundColor = ColorProvider(Color(0xFF243858)),
        )
    }

    private fun selectProvider(providers: List<ProviderUsage>, config: WidgetConfig): Pair<ProviderUsage, UsageQuota>? {
        if (config.mode != WIDGET_MODE_AUTO) {
            providers.firstOrNull { it.id == config.mode }?.let { provider ->
                Formatters.primaryQuota(provider, null)?.let { return provider to it }
            }
        }
        return Formatters.bestAvailableProvider(providers) { null }
            ?: providers.firstNotNullOfOrNull { provider -> Formatters.primaryQuota(provider, null)?.let { provider to it } }
    }
}

class UsageWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = UsageWidget()
}

/** Widget's own refresh icon: fetch, persist, and repaint — no separate
 * networking path from the app's. */
class RefreshWidgetAction : androidx.glance.appwidget.action.ActionCallback {
    override suspend fun onAction(context: Context, glanceId: GlanceId, parameters: androidx.glance.action.ActionParameters) {
        runCatching { UsageSyncRepository(context).refresh() }
        UsageWidget().update(context, glanceId)
    }
}
