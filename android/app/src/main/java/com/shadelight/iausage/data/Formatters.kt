package com.shadelight.iausage.data

import kotlin.math.roundToInt

/** Single source of truth for used/remaining/reset/age formatting. Dashboard
 * and Widget must never duplicate this math — a mismatch between the two
 * surfaces is exactly the kind of bug that only shows up in production. */
object Formatters {
    val ROUNDING_VECTORS: List<Triple<Double, Int, Int>> = listOf(
        Triple(5.48, 5, 95),
        Triple(5.50, 6, 94),
        Triple(14.88, 15, 85),
        Triple(38.2711, 38, 62),
        Triple(99.6, 100, 0),
        Triple(100.0, 100, 0),
        Triple(-1.0, 0, 100),
    )

    /** The quota that represents a provider when no per-provider preference
     * is set, or the preferred one falls back safely if its id disappeared. */
    fun primaryQuota(provider: ProviderUsage, preferredQuotaId: String?): UsageQuota? {
        if (preferredQuotaId != null) {
            provider.quotas.firstOrNull { it.id == preferredQuotaId }?.let { return it }
        }
        return provider.quotas.firstOrNull { isSummaryVisible(it.visible) } ?: provider.quotas.firstOrNull()
    }

    fun summaryPercents(usedExact: Double): Pair<Int, Int> {
        val used = if (usedExact.isNaN() || usedExact.isInfinite()) 0
        else usedExact.coerceIn(0.0, 100.0).roundToInt()
        return used to (100 - used)
    }

    fun availablePercent(usedPercent: Double?): Double? =
        usedPercent?.let { summaryPercents(it).second.toDouble() }

    fun percentText(usedPercent: Double?, usedMode: Boolean): String {
        val (used, remaining) = usedPercent?.let { summaryPercents(it) } ?: return "—"
        return "${if (usedMode) used else remaining}%"
    }

    fun percentPair(usedPercent: Double?): String {
        val (used, remaining) = usedPercent?.let { summaryPercents(it) } ?: return "—"
        return "$used% usado · $remaining% restante"
    }

    fun isSummaryVisible(visible: String?): Boolean = visible.isNullOrBlank() || visible == "always"

    fun windowLabel(quota: UsageQuota): String = when (quota.windowType ?: quota.label) {
        "weekly" -> "Semanal"
        "5h", "five_hour", "fiveHour" -> "5 horas"
        "session" -> "Sesión"
        "daily" -> "Diario"
        "monthly" -> "Mensual"
        else -> quota.label
    }

    fun groupsFromQuotas(quotas: List<UsageQuota>): List<QuotaGroup> {
        val groups = mutableListOf<QuotaGroup>()
        for (quota in quotas) {
            if (!isSummaryVisible(quota.visible) || quota.id == "total") continue
            val id = quota.groupId?.takeIf { it.isNotBlank() } ?: quota.id
            val existing = groups.firstOrNull { it.id == id }
            if (existing != null) {
                if (existing.label.isBlank() && !quota.groupLabel.isNullOrBlank()) existing.label = quota.groupLabel
                if (existing.models.isEmpty() && quota.models.isNotEmpty()) existing.models = quota.models
                existing.quotas.add(quota)
            } else {
                groups += QuotaGroup(
                    id = id,
                    label = quota.groupLabel?.takeIf { it.isNotBlank() } ?: quota.label,
                    models = quota.models,
                    quotas = mutableListOf(quota),
                )
            }
        }
        return groups
    }

    fun groupUsedExact(group: QuotaGroup): Double? =
        group.quotas.mapNotNull { it.usedPercent }.maxOrNull()

    fun groupSummary(group: QuotaGroup): Pair<Int, Int>? =
        groupUsedExact(group)?.let(::summaryPercents)

    /** "39 min", "4 h 31 min", "3 d 16 h 5 min" — never raw seconds. */
    fun formatDuration(seconds: Long): String {
        val s = seconds.coerceAtLeast(0)
        return when {
            s < 3_600 -> "${(s / 60).coerceAtLeast(1)} min"
            s < 86_400 -> {
                val hours = s / 3_600
                val minutes = (s % 3_600) / 60
                if (minutes > 0) "$hours h $minutes min" else "$hours h"
            }
            else -> {
                val days = s / 86_400
                val hours = (s % 86_400) / 3_600
                val minutes = (s % 3_600) / 60
                buildString {
                    append("$days d")
                    if (hours > 0 || minutes > 0) append(" $hours h")
                    if (minutes > 0) append(" $minutes min")
                }
            }
        }
    }

    /** "39m", "3h 50m", "4d 19h" — for the 2x1 widget, where only ~9
     * characters fit next to the percentage. */
    fun formatDurationShort(seconds: Long): String {
        val s = seconds.coerceAtLeast(0)
        return when {
            s < 3_600 -> "${(s / 60).coerceAtLeast(1)}m"
            s < 86_400 -> {
                val minutes = (s % 3_600) / 60
                if (minutes > 0) "${s / 3_600}h ${minutes}m" else "${s / 3_600}h"
            }
            else -> {
                val hours = (s % 86_400) / 3_600
                if (hours > 0) "${s / 86_400}d ${hours}h" else "${s / 86_400}d"
            }
        }
    }

    fun formatResetIn(seconds: Long?): String? = seconds?.let { "Reinicia en ${formatDuration(it)}" }

    /** Countdown derived from the absolute reset instant. `resetInSeconds`
     * belongs to the desktop snapshot generation time and would otherwise be
     * increasingly wrong while Android keeps the same cached payload. */
    fun secondsUntil(resetAt: String?, nowMillis: Long = System.currentTimeMillis()): Long? {
        val resetMillis = resetAt
            ?.let { runCatching { java.time.OffsetDateTime.parse(it).toInstant().toEpochMilli() }.getOrNull() }
            ?: return null
        return ((resetMillis - nowMillis) / 1000).coerceAtLeast(0)
    }

    fun liveResetSeconds(quota: UsageQuota, nowMillis: Long = System.currentTimeMillis()): Long? =
        secondsUntil(quota.resetAt, nowMillis) ?: quota.resetInSeconds

    /** "hace 12 s" / "hace 8 min" from an ISO-8601 instant. Never throws on a
     * malformed timestamp — worst case it reads as "hace un momento". */
    fun formatAge(iso: String?): String {
        val instantMillis = iso?.let { runCatching { java.time.Instant.parse(it).toEpochMilli() }.getOrNull() }
            ?: return "hace un momento"
        val seconds = ((System.currentTimeMillis() - instantMillis) / 1000).coerceAtLeast(0)
        return if (seconds < 5) "ahora" else "hace ${formatDuration(seconds)}"
    }

    fun isStale(provider: ProviderUsage): Boolean = provider.stale || provider.quotas.any { it.stale }

    fun isSnapshotStale(providers: List<ProviderUsage>): Boolean = providers.any(::isStale)

    /** The provider with the most room left, i.e. "best option right now"
     * for the user to switch to. Ties broken by catalog order. */
    fun bestAvailableProvider(providers: List<ProviderUsage>, preferredQuotaId: (String) -> String?): Pair<ProviderUsage, UsageQuota>? =
        providers
            .filter { it.enabled && (it.connectionStatus == null || it.connectionStatus == "connected") }
            .mapNotNull { provider -> primaryQuota(provider, preferredQuotaId(provider.id))?.let { provider to it } }
            .filter { it.second.usedPercent != null }
            .maxByOrNull { availablePercent(it.second.usedPercent) ?: 0.0 }
}

data class QuotaGroup(
    val id: String,
    var label: String,
    var models: List<String>,
    val quotas: MutableList<UsageQuota>,
)
