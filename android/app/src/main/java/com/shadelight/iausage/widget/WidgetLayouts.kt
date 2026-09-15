package com.shadelight.iausage.widget

import com.shadelight.iausage.data.Formatters
import com.shadelight.iausage.data.ProviderUsage
import com.shadelight.iausage.data.UsageQuota

/** One real layout per launcher footprint instead of a single layout scaled
 * up or down: small = one metric, medium = a short story, large = detail or
 * comparison. */
enum class WidgetLayout { TINY, WIDE, SQUARE, LARGE_SINGLE, LARGE_COMPARE }

/** Breakpoints sit between cell counts: one cell is ~57 dp, two ~130 dp and
 * four ~250 dp on typical launchers. A narrow widget is always TINY, even if
 * it is tall: there is no width for a second column of text. */
fun widgetLayout(widthDp: Float, heightDp: Float, mode: String): WidgetLayout = when {
    widthDp < 110f -> WidgetLayout.TINY
    heightDp < 110f -> WidgetLayout.WIDE
    widthDp < 240f -> WidgetLayout.SQUARE
    mode == WIDGET_MODE_COMPARE -> WidgetLayout.LARGE_COMPARE
    else -> WidgetLayout.LARGE_SINGLE
}

enum class UsageLevel { OK, WARN, CRITICAL, UNKNOWN }

/** Same bands as the desktop notification thresholds (75 / 90). */
fun usageLevel(usedPercent: Double?): UsageLevel = when {
    usedPercent == null -> UsageLevel.UNKNOWN
    usedPercent < 75.0 -> UsageLevel.OK
    usedPercent < 90.0 -> UsageLevel.WARN
    else -> UsageLevel.CRITICAL
}

fun activeProviders(providers: List<ProviderUsage>): List<ProviderUsage> = providers.filter { it.enabled }

/** Single-provider layouts: the fixed provider while it still has data,
 * otherwise the one with the most room left. */
fun pickProvider(providers: List<ProviderUsage>, mode: String): Pair<ProviderUsage, UsageQuota>? {
    if (mode != WIDGET_MODE_AUTO && mode != WIDGET_MODE_COMPARE) {
        providers.firstOrNull { it.id == mode }?.let { provider ->
            Formatters.primaryQuota(provider, null)?.let { return provider to it }
        }
    }
    return Formatters.bestAvailableProvider(providers) { null }
        ?: providers.firstNotNullOfOrNull { provider -> Formatters.primaryQuota(provider, null)?.let { provider to it } }
}

const val MAX_COMPARE_ROWS = 3

data class CompareRows(val rows: List<ProviderUsage>, val hiddenCount: Int)

/** At most three rows so names and percentages never get squeezed together;
 * the rest collapse into "+N más". An empty selection means "all". */
fun compareRows(providers: List<ProviderUsage>, visibleIds: Set<String>): CompareRows {
    val candidates = providers.filter { it.enabled && (visibleIds.isEmpty() || it.id in visibleIds) }
    return CompareRows(candidates.take(MAX_COMPARE_ROWS), (candidates.size - MAX_COMPARE_ROWS).coerceAtLeast(0))
}

fun metricWord(config: WidgetConfig): String = if (config.usedMode) "usado" else "restante"

/** "Semanal · 2 h" — the reset rides on the label line so a two-quota large
 * widget still fits in a 4x2 footprint. */
fun quotaCaption(quota: UsageQuota, config: WidgetConfig): String {
    val label = Formatters.windowLabel(quota)
    val resetIn = Formatters.liveResetSeconds(quota)
    return if (config.showReset && resetIn != null) "$label · ${Formatters.formatDuration(resetIn)}" else label
}

/** Bottom line: time to reset and/or data age, joined so it takes one row.
 * `compact` (2x2) keeps only the first part: both never fit in two cells and
 * a truncated "Reinicia en 3 h 50 min ·…" helps nobody. A stale warning still
 * wins over the reset, because old data is the more important fact. */
fun widgetFooter(quota: UsageQuota?, config: WidgetConfig, generatedAt: String?, stale: Boolean, compact: Boolean = false): String {
    val reset = quota?.let(Formatters::liveResetSeconds)?.takeIf { config.showReset }?.let { "Reinicia en ${Formatters.formatDuration(it)}" }
    val age = generatedAt?.takeIf { config.showStatus }?.let { at ->
        if (stale) "⚠ datos de ${Formatters.formatAge(at)}" else Formatters.formatAge(at)
    }
    if (compact) return (if (stale) age ?: reset else reset ?: age).orEmpty()
    return listOfNotNull(reset, age).joinToString(" · ")
}

/** The only required choice in the configurator: a single-provider widget
 * needs its provider. Everything else has a sensible default. */
fun widgetDraftError(singleProvider: Boolean, providerId: String?): String? =
    if (singleProvider && providerId == null) "Selecciona una IA" else null
