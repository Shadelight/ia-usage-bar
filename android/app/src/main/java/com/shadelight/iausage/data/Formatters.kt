package com.shadelight.iausage.data

/** Single source of truth for used/available/reset/age formatting. Dashboard
 * and Widget must never duplicate this math — a mismatch between the two
 * surfaces is exactly the kind of bug that only shows up in production. */
object Formatters {
    /** The quota that represents a provider when no per-provider preference
     * is set, or the preferred one falls back safely if its id disappeared. */
    fun primaryQuota(provider: ProviderUsage, preferredQuotaId: String?): UsageQuota? {
        if (preferredQuotaId != null) {
            provider.quotas.firstOrNull { it.id == preferredQuotaId }?.let { return it }
        }
        return provider.quotas.firstOrNull()
    }

    fun availablePercent(usedPercent: Double?): Double? =
        usedPercent?.let { (100.0 - it).coerceIn(0.0, 100.0) }

    fun percentText(usedPercent: Double?, usedMode: Boolean): String {
        val value = if (usedMode) usedPercent else availablePercent(usedPercent)
        return value?.let { "${it.toInt()}%" } ?: "—"
    }

    /** "39 min", "4 h 31 min", "3 d", "6 d 7 h" — never raw seconds. */
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
                if (hours > 0) "$days d $hours h" else "$days d"
            }
        }
    }

    fun formatResetIn(seconds: Long?): String? = seconds?.let { "Reinicia en ${formatDuration(it)}" }

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

    /** The provider with the least room left, i.e. "best option right now"
     * for the user to switch to. Ties broken by catalog order. */
    fun bestAvailableProvider(providers: List<ProviderUsage>, preferredQuotaId: (String) -> String?): Pair<ProviderUsage, UsageQuota>? =
        providers
            .filter { it.enabled && (it.connectionStatus == null || it.connectionStatus == "connected") }
            .mapNotNull { provider -> primaryQuota(provider, preferredQuotaId(provider.id))?.let { provider to it } }
            .filter { it.second.usedPercent != null }
            .maxByOrNull { availablePercent(it.second.usedPercent) ?: 0.0 }
}
