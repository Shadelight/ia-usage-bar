package com.shadelight.iausage.alerts

import com.shadelight.iausage.data.Formatters
import com.shadelight.iausage.data.ProviderUsage
import org.json.JSONArray
import org.json.JSONObject
import java.time.OffsetDateTime
import kotlin.math.roundToInt

data class ProviderAlertState(
    val resetAt: String?,
    val usedPercent: Double?,
    val notified: Set<Int> = emptySet(),
)

data class AlertState(
    val providers: Map<String, ProviderAlertState> = emptyMap(),
    val bestId: String? = null,
    val syncFailures: Int = 0,
    val syncAlerted: Boolean = false,
)

/** What the user is told. `key` doubles as the notification id, so a newer
 * alert of the same kind for the same provider replaces the older one
 * instead of stacking. */
sealed class UsageAlert(val key: String, val title: String, val body: String) {
    class Reset(providerId: String, name: String, availablePercent: Int) :
        UsageAlert("reset:$providerId", "$name se reinició", "$availablePercent% disponible.")

    class NearLimit(providerId: String, name: String, usedPercent: Int) :
        UsageAlert("limit:$providerId", "$name está por agotarse", "Llevas el $usedPercent% usado.")

    class Exhausted(providerId: String, name: String) :
        UsageAlert("limit:$providerId", "$name se agotó", "Llegaste al 100% de la cuota.")

    class BestChanged(name: String, availablePercent: Int) :
        UsageAlert("best", "Ahora $name es la mejor opción", "$availablePercent% disponible.")

    object SyncLost : UsageAlert(
        "sync",
        "No se puede contactar con el PC",
        "Revisa que IA Usage siga abierto en el PC y en la misma red.",
    )
}

/** Pure transition logic, a port of the desktop's notification rules: every
 * refresh path (foreground service, periodic worker, widget refresh) feeds
 * it the latest snapshot and posts whatever it returns. */
object UsageAlerts {
    const val NEAR_LIMIT_PERCENT = 90
    const val EXHAUSTED_PERCENT = 100

    /** Same anti-flapping margin as the desktop recommendation's SWITCH_MARGIN. */
    const val BEST_SWITCH_MARGIN = 15.0

    /** Three missed polls (~9 min with the service) before calling the PC lost:
     * a single dropped request on Wi-Fi is not worth a notification. */
    const val SYNC_FAILURES_BEFORE_ALERT = 3

    fun onRefreshed(state: AlertState, providers: List<ProviderUsage>): Pair<List<UsageAlert>, AlertState> {
        val alerts = mutableListOf<UsageAlert>()
        val next = state.providers.toMutableMap()
        for (provider in providers.filter(::isUsable)) {
            val quota = Formatters.primaryQuota(provider, null) ?: continue
            val used = quota.usedPercent ?: continue
            val previous = state.providers[provider.id]
            if (previous == null) {
                // First sighting: remember it, never announce a transition we didn't observe.
                next[provider.id] = ProviderAlertState(quota.resetAt, used)
                continue
            }
            val reset = resetHappened(previous.resetAt, quota.resetAt, previous.usedPercent, used)
            var notified = if (reset) emptySet() else previous.notified
            if (reset) {
                alerts += UsageAlert.Reset(provider.id, provider.name, (100.0 - used).roundToInt().coerceIn(0, 100))
            }
            val before = if (reset) 0.0 else previous.usedPercent ?: 0.0
            val exhausted = crossed(before, used, EXHAUSTED_PERCENT - 0.5) && EXHAUSTED_PERCENT !in notified
            val nearLimit = crossed(before, used, NEAR_LIMIT_PERCENT.toDouble()) && NEAR_LIMIT_PERCENT !in notified
            if (exhausted) {
                alerts += UsageAlert.Exhausted(provider.id, provider.name)
                notified = notified + EXHAUSTED_PERCENT + NEAR_LIMIT_PERCENT
            } else if (nearLimit) {
                alerts += UsageAlert.NearLimit(provider.id, provider.name, used.roundToInt())
                notified = notified + NEAR_LIMIT_PERCENT
            }
            next[provider.id] = ProviderAlertState(quota.resetAt, used, notified)
        }
        val bestId = bestAfter(state.bestId, providers, alerts)
        return alerts to state.copy(providers = next, bestId = bestId, syncFailures = 0, syncAlerted = false)
    }

    fun onRefreshFailed(state: AlertState): Pair<List<UsageAlert>, AlertState> {
        val failures = state.syncFailures + 1
        val alert = failures >= SYNC_FAILURES_BEFORE_ALERT && !state.syncAlerted
        val alerts = if (alert) listOf<UsageAlert>(UsageAlert.SyncLost) else emptyList()
        return alerts to state.copy(syncFailures = failures, syncAlerted = state.syncAlerted || alert)
    }

    /** A discrete window reset jumps `resetAt` forward; a sliding window
     * creeps it forward on every poll without clearing. Require usage to
     * drop too, exactly like the desktop, or every poll would be a "reset". */
    internal fun resetHappened(previousResetAt: String?, resetAt: String?, previousUsed: Double?, used: Double): Boolean {
        val before = epochSeconds(previousResetAt) ?: return false
        val after = epochSeconds(resetAt) ?: return false
        return after - before > 120 && (previousUsed == null || used < previousUsed)
    }

    private fun bestAfter(previousId: String?, providers: List<ProviderUsage>, alerts: MutableList<UsageAlert>): String? {
        val (best, quota) = Formatters.bestAvailableProvider(providers) { null } ?: return previousId
        if (previousId == null || best.id == previousId) return best.id
        val available = Formatters.availablePercent(quota.usedPercent) ?: return previousId
        val previousAvailable = providers.firstOrNull { it.id == previousId && isUsable(it) }
            ?.let { Formatters.availablePercent(Formatters.primaryQuota(it, null)?.usedPercent) }
        if (previousAvailable != null && available - previousAvailable < BEST_SWITCH_MARGIN) return previousId
        alerts += UsageAlert.BestChanged(best.name, available.roundToInt())
        return best.id
    }

    private fun crossed(before: Double, now: Double, threshold: Double) = before < threshold && now >= threshold

    private fun isUsable(provider: ProviderUsage) =
        provider.enabled && (provider.connectionStatus == null || provider.connectionStatus == "connected")

    private fun epochSeconds(iso: String?): Long? =
        iso?.let { runCatching { OffsetDateTime.parse(it).toEpochSecond() }.getOrNull() }
}

internal object AlertStateCodec {
    fun encode(state: AlertState): String = JSONObject()
        .put("bestId", state.bestId ?: JSONObject.NULL)
        .put("syncFailures", state.syncFailures)
        .put("syncAlerted", state.syncAlerted)
        .put(
            "providers",
            JSONObject().also { all ->
                state.providers.forEach { (id, provider) ->
                    all.put(
                        id,
                        JSONObject()
                            .put("resetAt", provider.resetAt ?: JSONObject.NULL)
                            .put("usedPercent", provider.usedPercent ?: JSONObject.NULL)
                            .put("notified", JSONArray(provider.notified.sorted())),
                    )
                }
            },
        )
        .toString()

    /** Corrupt or missing state just starts over: worst case one transition
     * goes unannounced, never a crash in the service. */
    fun decode(raw: String?): AlertState = runCatching {
        val json = JSONObject(raw ?: return AlertState())
        val providers = json.optJSONObject("providers") ?: JSONObject()
        AlertState(
            providers = providers.keys().asSequence().associateWith { id ->
                val provider = providers.getJSONObject(id)
                val notified = provider.optJSONArray("notified") ?: JSONArray()
                ProviderAlertState(
                    resetAt = if (provider.isNull("resetAt")) null else provider.getString("resetAt"),
                    usedPercent = if (provider.isNull("usedPercent")) null else provider.getDouble("usedPercent"),
                    notified = (0 until notified.length()).map { notified.getInt(it) }.toSet(),
                )
            },
            bestId = if (json.isNull("bestId")) null else json.getString("bestId"),
            syncFailures = json.optInt("syncFailures", 0),
            syncAlerted = json.optBoolean("syncAlerted", false),
        )
    }.getOrDefault(AlertState())
}
