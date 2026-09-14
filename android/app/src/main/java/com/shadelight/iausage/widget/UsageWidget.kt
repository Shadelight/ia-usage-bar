package com.shadelight.iausage.widget

import android.content.Context
import android.content.Intent
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.datastore.preferences.core.Preferences
import androidx.glance.ColorFilter
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.Image
import androidx.glance.ImageProvider
import androidx.glance.LocalContext
import androidx.glance.LocalSize
import androidx.glance.action.ActionParameters
import androidx.glance.action.clickable
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.LinearProgressIndicator
import androidx.glance.appwidget.SizeMode
import androidx.glance.appwidget.action.ActionCallback
import androidx.glance.appwidget.action.actionRunCallback
import androidx.glance.appwidget.action.actionStartActivity
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.provideContent
import androidx.glance.appwidget.updateAll
import androidx.glance.background
import androidx.glance.currentState
import androidx.glance.layout.Alignment
import androidx.glance.layout.Box
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
import com.shadelight.iausage.alerts.AlertPipeline
import com.shadelight.iausage.data.Formatters
import com.shadelight.iausage.data.ProviderUsage
import com.shadelight.iausage.data.SecureStateStore
import com.shadelight.iausage.data.SyncPayload
import com.shadelight.iausage.data.UsageQuota
import com.shadelight.iausage.data.UsageSyncRepository
import com.shadelight.iausage.data.providerShortName
import com.shadelight.iausage.data.providerVisual
import org.json.JSONObject

// One size per launcher footprint (see widgetLayout for the breakpoints).
private val TinySize = DpSize(57.dp, 57.dp)
private val WideSize = DpSize(130.dp, 57.dp)
private val SquareSize = DpSize(130.dp, 130.dp)
private val LargeSize = DpSize(250.dp, 130.dp)
private val LargeTallSize = DpSize(250.dp, 200.dp)

private fun textStyle(color: Color, size: Int, weight: FontWeight = FontWeight.Normal) =
    TextStyle(color = ColorProvider(color), fontSize = size.sp, fontWeight = weight)

class UsageWidget : GlanceAppWidget() {
    override val stateDefinition = PreferencesGlanceStateDefinition
    override val sizeMode = SizeMode.Responsive(setOf(TinySize, WideSize, SquareSize, LargeSize, LargeTallSize))

    override suspend fun provideGlance(context: Context, id: GlanceId) {
        // A widget host (notably Samsung One UI) shows a permanent "can't
        // load widget" placeholder if provideGlance ever throws, so any
        // failure here must degrade to an empty state instead of crashing.
        val payload = runCatching {
            SecureStateStore(context).payload()?.let { runCatching { SyncPayload.fromJson(JSONObject(it)) }.getOrNull() }
        }.getOrNull()
        provideContent { WidgetContent(payload) }
    }

    @Composable
    private fun WidgetContent(payload: SyncPayload?) {
        val size = LocalSize.current
        val config = readWidgetConfig(currentState<Preferences>())
        val context = LocalContext.current
        val layout = widgetLayout(size.width.value, size.height.value, config.mode)
        val providers = payload?.snapshot?.providers?.let(::activeProviders).orEmpty()

        Box(
            modifier = GlanceModifier
                .fillMaxSize()
                .background(WidgetPalette.Background)
                .cornerRadius(20.dp)
                .clickable(actionStartActivity(Intent(context, MainActivity::class.java)))
                .padding(if (layout == WidgetLayout.TINY) 6.dp else 14.dp),
        ) {
            when {
                payload == null -> EmptyContent(layout, "Vincula tu PC", "Abre IA Usage")
                providers.isEmpty() -> EmptyContent(layout, "Sin IAs activas", "Actívalas en el PC")
                layout == WidgetLayout.LARGE_COMPARE -> CompareContent(payload, providers, config, size.height)
                else -> {
                    val picked = pickProvider(providers, config.mode)
                    if (picked == null) {
                        EmptyContent(layout, "Sin datos todavía", "Actualiza desde el PC")
                    } else {
                        val (provider, quota) = picked
                        val stale = Formatters.isStale(provider)
                        when (layout) {
                            WidgetLayout.TINY -> TinyContent(provider, quota, config)
                            WidgetLayout.WIDE -> WideContent(provider, quota, config)
                            WidgetLayout.SQUARE -> SquareContent(payload, provider, quota, config, stale)
                            else -> LargeSingleContent(payload, provider, config, stale)
                        }
                    }
                }
            }
        }
    }

    /** 1x1: status dot + short code + one big number. Nothing else fits. */
    @Composable
    private fun TinyContent(provider: ProviderUsage, quota: UsageQuota, config: WidgetConfig) {
        Column(
            modifier = GlanceModifier.fillMaxSize(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                StatusDot(quota.usedPercent)
                Spacer(GlanceModifier.width(4.dp))
                Text(
                    text = providerVisual(provider.id, provider.name).shortCode,
                    style = textStyle(WidgetPalette.TextMuted, 10, FontWeight.Medium),
                    maxLines = 1,
                )
            }
            Text(
                text = Formatters.percentText(quota.usedPercent, config.usedMode),
                style = textStyle(WidgetPalette.TextPrimary, 19, FontWeight.Bold),
                maxLines = 1,
            )
        }
    }

    /** 2x1: provider + main metric + time to reset. */
    @Composable
    private fun WideContent(provider: ProviderUsage, quota: UsageQuota, config: WidgetConfig) {
        Row(modifier = GlanceModifier.fillMaxSize(), verticalAlignment = Alignment.CenterVertically) {
            ProviderIcon(provider, 22)
            Spacer(GlanceModifier.width(10.dp))
            Column(modifier = GlanceModifier.defaultWeight()) {
                Text(
                    text = providerShortName(provider.id, provider.name),
                    style = textStyle(WidgetPalette.TextPrimary, 13, FontWeight.Medium),
                    maxLines = 1,
                )
                quota.resetInSeconds?.takeIf { config.showReset }?.let {
                    // ~9 characters fit next to the number in two cells.
                    Text(
                        text = "↻ ${Formatters.formatDurationShort(it)}",
                        style = textStyle(WidgetPalette.TextMuted, 11),
                        maxLines = 1,
                    )
                }
            }
            Spacer(GlanceModifier.width(8.dp))
            Column(horizontalAlignment = Alignment.End) {
                Text(
                    text = Formatters.percentText(quota.usedPercent, config.usedMode),
                    style = textStyle(WidgetPalette.TextPrimary, 20, FontWeight.Bold),
                    maxLines = 1,
                )
                Text(text = metricWord(config), style = textStyle(WidgetPalette.TextMuted, 10), maxLines = 1)
            }
        }
    }

    /** 2x2: provider, big number, bar, reset + age, refresh. */
    @Composable
    private fun SquareContent(payload: SyncPayload, provider: ProviderUsage, quota: UsageQuota, config: WidgetConfig, stale: Boolean) {
        Column(modifier = GlanceModifier.fillMaxSize()) {
            Row(modifier = GlanceModifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                ProviderIcon(provider, 18)
                Spacer(GlanceModifier.width(8.dp))
                Text(
                    text = providerShortName(provider.id, provider.name),
                    modifier = GlanceModifier.defaultWeight(),
                    style = textStyle(WidgetPalette.TextPrimary, 13, FontWeight.Medium),
                    maxLines = 1,
                )
                StatusDot(quota.usedPercent)
            }
            Spacer(GlanceModifier.defaultWeight())
            Row(verticalAlignment = Alignment.Bottom) {
                Text(
                    text = Formatters.percentText(quota.usedPercent, config.usedMode),
                    style = textStyle(WidgetPalette.TextPrimary, 30, FontWeight.Bold),
                    maxLines = 1,
                )
                Spacer(GlanceModifier.width(4.dp))
                Text(
                    text = metricWord(config),
                    modifier = GlanceModifier.padding(bottom = 5.dp),
                    style = textStyle(WidgetPalette.TextMuted, 11),
                    maxLines = 1,
                )
            }
            if (config.showBar) {
                Spacer(GlanceModifier.height(8.dp))
                UsageBar(quota.usedPercent)
            }
            Spacer(GlanceModifier.height(8.dp))
            FooterRow(widgetFooter(quota, config, payload.generatedAt, stale, compact = true), stale)
        }
    }

    /** 4x2, one provider: session and weekly with their own bars. */
    @Composable
    private fun LargeSingleContent(payload: SyncPayload, provider: ProviderUsage, config: WidgetConfig, stale: Boolean) {
        Column(modifier = GlanceModifier.fillMaxSize()) {
            Row(modifier = GlanceModifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                ProviderIcon(provider, 20)
                Spacer(GlanceModifier.width(8.dp))
                Text(
                    text = provider.name,
                    modifier = GlanceModifier.defaultWeight(),
                    style = textStyle(WidgetPalette.TextPrimary, 14, FontWeight.Medium),
                    maxLines = 1,
                )
                StatusDot(Formatters.primaryQuota(provider, null)?.usedPercent)
            }
            // Glance caps a Column at 10 children (extra ones are silently
            // dropped), so each quota is its own Column.
            provider.quotas.take(2).forEach { quota ->
                Column(modifier = GlanceModifier.fillMaxWidth().padding(top = 9.dp)) {
                    Row(modifier = GlanceModifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            text = quotaCaption(quota, config),
                            modifier = GlanceModifier.defaultWeight(),
                            style = textStyle(WidgetPalette.TextMuted, 12),
                            maxLines = 1,
                        )
                        Spacer(GlanceModifier.width(8.dp))
                        Text(
                            text = Formatters.percentText(quota.usedPercent, config.usedMode),
                            style = textStyle(WidgetPalette.TextPrimary, 15, FontWeight.Bold),
                            maxLines = 1,
                        )
                    }
                    if (config.showBar) {
                        Spacer(GlanceModifier.height(5.dp))
                        UsageBar(quota.usedPercent)
                    }
                }
            }
            Spacer(GlanceModifier.defaultWeight())
            FooterRow(widgetFooter(null, config, payload.generatedAt, stale), stale)
        }
    }

    /** 4x2, comparison: one row per provider, name left, value right, bar
     * below, three rows max and "+N más" for the rest. */
    @Composable
    private fun CompareContent(payload: SyncPayload, providers: List<ProviderUsage>, config: WidgetConfig, height: Dp) {
        val (rows, hidden) = compareRows(providers, config.visibleProviderIds)
        val roomy = height >= LargeTallSize.height
        Column(modifier = GlanceModifier.fillMaxSize()) {
            Row(modifier = GlanceModifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Image(
                    provider = ImageProvider(R.drawable.ic_ia_usage_mark),
                    contentDescription = null,
                    modifier = GlanceModifier.size(16.dp),
                )
                Spacer(GlanceModifier.width(6.dp))
                Text(
                    text = "Tus IAs",
                    modifier = GlanceModifier.defaultWeight(),
                    style = textStyle(WidgetPalette.TextPrimary, 13, FontWeight.Medium),
                    maxLines = 1,
                )
                if (hidden > 0) {
                    Text(text = "+$hidden más", style = textStyle(WidgetPalette.TextMuted, 11), maxLines = 1)
                    Spacer(GlanceModifier.width(8.dp))
                }
                RefreshButton()
            }
            // One Column per row: Glance caps a Column at 10 children.
            rows.forEach { provider ->
                val quota = Formatters.primaryQuota(provider, null)
                Column(modifier = GlanceModifier.fillMaxWidth().padding(top = if (roomy) 12.dp else 6.dp)) {
                    Row(modifier = GlanceModifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                        ProviderIcon(provider, 16)
                        Spacer(GlanceModifier.width(8.dp))
                        Text(
                            text = providerShortName(provider.id, provider.name),
                            modifier = GlanceModifier.defaultWeight(),
                            style = textStyle(WidgetPalette.TextPrimary, 12),
                            maxLines = 1,
                        )
                        Spacer(GlanceModifier.width(8.dp))
                        Text(
                            text = Formatters.percentText(quota?.usedPercent, config.usedMode),
                            style = textStyle(WidgetPalette.TextPrimary, 13, FontWeight.Bold),
                            maxLines = 1,
                        )
                    }
                    if (config.showBar) {
                        Spacer(GlanceModifier.height(3.dp))
                        UsageBar(quota?.usedPercent, thin = true)
                    }
                }
            }
            if (config.showStatus) {
                val stale = Formatters.isSnapshotStale(rows)
                Spacer(GlanceModifier.defaultWeight())
                Text(
                    text = widgetFooter(null, config.copy(showReset = false), payload.generatedAt, stale),
                    style = textStyle(if (stale) WidgetPalette.Warn else WidgetPalette.TextMuted, 10),
                    maxLines = 1,
                )
            }
        }
    }

    @Composable
    private fun EmptyContent(layout: WidgetLayout, title: String, subtitle: String) {
        Column(
            modifier = GlanceModifier.fillMaxSize(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Image(
                provider = ImageProvider(R.drawable.ic_ia_usage_mark),
                contentDescription = null,
                modifier = GlanceModifier.size(if (layout == WidgetLayout.TINY) 22.dp else 26.dp),
            )
            if (layout != WidgetLayout.TINY) {
                Spacer(GlanceModifier.height(6.dp))
                Text(text = title, style = textStyle(WidgetPalette.TextPrimary, 13, FontWeight.Medium), maxLines = 1)
                if (layout != WidgetLayout.WIDE) {
                    Text(text = subtitle, style = textStyle(WidgetPalette.TextMuted, 11), maxLines = 2)
                }
            }
        }
    }

    @Composable
    private fun FooterRow(text: String, stale: Boolean) {
        Row(modifier = GlanceModifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(
                text = text,
                modifier = GlanceModifier.defaultWeight(),
                style = textStyle(if (stale) WidgetPalette.Warn else WidgetPalette.TextMuted, 11),
                maxLines = 1,
            )
            RefreshButton()
        }
    }

    /** Quiet circular button with the IA Usage glyph, same weight as the
     * rest of the card instead of the system sync icon. */
    @Composable
    private fun RefreshButton() {
        Box(
            modifier = GlanceModifier
                .size(26.dp)
                .cornerRadius(13.dp)
                .background(WidgetPalette.Surface)
                .clickable(actionRunCallback<RefreshWidgetAction>()),
            contentAlignment = Alignment.Center,
        ) {
            Image(
                provider = ImageProvider(R.drawable.ic_widget_refresh),
                contentDescription = "Actualizar",
                modifier = GlanceModifier.size(13.dp),
                colorFilter = ColorFilter.tint(ColorProvider(WidgetPalette.TextMuted)),
            )
        }
    }

    @Composable
    private fun UsageBar(usedPercent: Double?, thin: Boolean = false) {
        LinearProgressIndicator(
            progress = ((usedPercent ?: 0.0) / 100.0).toFloat().coerceIn(0f, 1f),
            modifier = GlanceModifier.fillMaxWidth().height(if (thin) 3.dp else 6.dp),
            color = ColorProvider(WidgetPalette.bar(usageLevel(usedPercent))),
            backgroundColor = ColorProvider(WidgetPalette.Track),
        )
    }

    @Composable
    private fun StatusDot(usedPercent: Double?) {
        Box(
            modifier = GlanceModifier
                .size(8.dp)
                .cornerRadius(4.dp)
                .background(WidgetPalette.status(usageLevel(usedPercent))),
        ) {}
    }

    @Composable
    private fun ProviderIcon(provider: ProviderUsage, sizeDp: Int) {
        val visual = providerVisual(provider.id, provider.name)
        val icon = visual.icon
        if (icon != null) {
            // Brand marks ship dark for the light app theme; on the dark card
            // they are tinted light, like the desktop inverts them in dark mode.
            Image(
                provider = ImageProvider(icon),
                contentDescription = null,
                modifier = GlanceModifier.size(sizeDp.dp),
                colorFilter = ColorFilter.tint(ColorProvider(WidgetPalette.TextPrimary)),
            )
        } else {
            Text(text = visual.shortCode, style = textStyle(WidgetPalette.TextMuted, 10, FontWeight.Bold), maxLines = 1)
        }
    }
}

class UsageWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = UsageWidget()
}

/** Widget's own refresh button: fetch, persist, feed the alert pipeline and
 * repaint every instance — no separate networking path from the app's. */
class RefreshWidgetAction : ActionCallback {
    override suspend fun onAction(context: Context, glanceId: GlanceId, parameters: ActionParameters) {
        runCatching { UsageSyncRepository(context).refresh() }
            .onSuccess { AlertPipeline.onRefreshed(context, it) }
        UsageWidget().updateAll(context)
    }
}
