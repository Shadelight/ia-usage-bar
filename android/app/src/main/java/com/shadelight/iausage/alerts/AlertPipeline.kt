package com.shadelight.iausage.alerts

import android.content.Context
import com.shadelight.iausage.data.SyncPayload

/** Persisted transition state: JSON in private SharedPreferences. */
class AlertStore(context: Context) {
    private val prefs = context.applicationContext.getSharedPreferences("iausage.alerts", Context.MODE_PRIVATE)

    fun load(): AlertState = AlertStateCodec.decode(prefs.getString(KEY, null))

    fun save(state: AlertState) {
        prefs.edit().putString(KEY, AlertStateCodec.encode(state)).apply()
    }

    private companion object {
        const val KEY = "state"
    }
}

/** Opt-in switch for background alerts + the monitoring service. Plain
 * SharedPreferences rather than the UI DataStore because the boot receiver
 * needs a synchronous read. */
object MonitorSettings {
    private const val KEY_ENABLED = "enabled"

    private fun prefs(context: Context) =
        context.applicationContext.getSharedPreferences("iausage.monitor", Context.MODE_PRIVATE)

    fun isEnabled(context: Context): Boolean = prefs(context).getBoolean(KEY_ENABLED, false)

    fun setEnabled(context: Context, enabled: Boolean) {
        prefs(context).edit().putBoolean(KEY_ENABLED, enabled).apply()
        if (enabled) MonitorService.start(context) else MonitorService.stop(context)
    }
}

/** Single entry point for every refresh path (foreground service, periodic
 * worker, widget refresh button). The lock keeps two paths finishing at the
 * same time from both notifying the same transition. State is tracked even
 * while alerts are off, so turning them on never replays old transitions. */
object AlertPipeline {
    private val lock = Any()

    fun onRefreshed(context: Context, payload: SyncPayload) = synchronized(lock) {
        val store = AlertStore(context)
        val (alerts, next) = UsageAlerts.onRefreshed(store.load(), payload.snapshot.providers)
        store.save(next)
        if (MonitorSettings.isEnabled(context)) alerts.forEach { Notifier.post(context, it) }
    }

    fun onRefreshFailed(context: Context) = synchronized(lock) {
        val store = AlertStore(context)
        val (alerts, next) = UsageAlerts.onRefreshFailed(store.load())
        store.save(next)
        if (MonitorSettings.isEnabled(context)) alerts.forEach { Notifier.post(context, it) }
    }
}
