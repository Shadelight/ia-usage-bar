package com.shadelight.iausage.debug

import android.appwidget.AppWidgetManager
import android.content.BroadcastReceiver
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import androidx.glance.appwidget.updateAll
import com.shadelight.iausage.alerts.AlertPipeline
import com.shadelight.iausage.alerts.MonitorSettings
import com.shadelight.iausage.data.PairingInfo
import com.shadelight.iausage.data.SecureStateStore
import com.shadelight.iausage.data.SyncPayload
import com.shadelight.iausage.widget.UsageWidget
import com.shadelight.iausage.widget.UsageWidgetReceiver
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import org.json.JSONArray
import org.json.JSONObject
import java.time.Instant
import java.time.temporal.ChronoUnit

/** Debug builds only (src/debug): seeds a realistic snapshot so the widget,
 * its configurator and the alerts can be checked on an emulator without a
 * paired PC. Each broadcast also runs the alert pipeline, so a sequence of
 * broadcasts replays transitions.
 *
 *   adb shell am broadcast -n com.shadelight.iausage/.debug.DemoDataReceiver \
 *     --es claude 64 --es claudeResetMinutes 178 --es codex 0 --ez alerts true --ez pin true
 */
class DemoDataReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val pending = goAsync()
        val claude = intent.getStringExtra("claude")?.toDoubleOrNull() ?: 64.0
        val claudeResetMinutes = intent.getStringExtra("claudeResetMinutes")?.toLongOrNull() ?: 178L
        val codex = intent.getStringExtra("codex")?.toDoubleOrNull() ?: 0.0
        val json = demoPayload(claude, claudeResetMinutes, codex)
        val store = SecureStateStore(context)
        store.savePayload(json)
        if (intent.getBooleanExtra("pairDemo", false)) {
            // A fake PC on the emulator host, only so the foreground monitor has
            // "a pairing" to run against; its polls fail and exercise that path.
            store.savePairing(PairingInfo("10.0.2.2", 28741, "demo-device", "DEMO-0000", "0.3.0"), "demo".toCharArray())
        }
        if (intent.getBooleanExtra("alerts", false) && !MonitorSettings.isEnabled(context)) {
            MonitorSettings.setEnabled(context, true)
        }
        runCatching { AlertPipeline.onRefreshed(context, SyncPayload.fromJson(JSONObject(json))) }
        if (intent.getBooleanExtra("pin", false)) {
            runCatching {
                context.getSystemService(AppWidgetManager::class.java)
                    .requestPinAppWidget(ComponentName(context, UsageWidgetReceiver::class.java), null, null)
            }
        }
        CoroutineScope(Dispatchers.Default).launch {
            runCatching { UsageWidget().updateAll(context) }
            pending.finish()
        }
    }

    private fun demoPayload(claude: Double, claudeResetMinutes: Long, codex: Double): String {
        val now = Instant.now()
        fun quota(id: String, label: String, used: Double?, resetMinutes: Long?) = JSONObject()
            .put("id", id)
            .put("label", label)
            .put("usedPercent", used ?: JSONObject.NULL)
            .put("resetAt", resetMinutes?.let { now.plus(it, ChronoUnit.MINUTES).toString() } ?: JSONObject.NULL)
            .put("resetInSeconds", resetMinutes?.let { it * 60 } ?: JSONObject.NULL)
            .put("stale", false)
        fun provider(id: String, name: String, status: String, vararg quotas: JSONObject) = JSONObject()
            .put("id", id)
            .put("name", name)
            .put("enabled", true)
            .put("connection", JSONObject().put("status", status).put("reason", JSONObject.NULL))
            .put("stale", false)
            .put("updatedAt", now.toString())
            .put("quotas", JSONArray(quotas.toList()))
        val providers = JSONArray(
            listOf(
                provider(
                    "anthropic", "Claude Code", "connected",
                    quota("session", "Sesión", claude, claudeResetMinutes),
                    quota("weekly", "Semanal", 92.0, 2_340),
                ),
                provider(
                    "openai", "Codex / ChatGPT", "connected",
                    quota("5h", "5 horas", codex, 230),
                    quota("weekly", "Semanal", 59.0, 6_900),
                ),
                provider("cursor", "Cursor", "connected", quota("total", "Uso total", 40.0, 12_500)),
                provider("copilot", "GitHub Copilot", "connected", quota("premium", "Premium", 100.0, 23_000)),
                provider("antigravity", "Antigravity", "needs_auth"),
            ),
        )
        val generatedAt = now.minus(3, ChronoUnit.MINUTES).toString()
        return JSONObject()
            .put("schemaVersion", 1)
            .put("deviceId", "demo-device")
            .put("generatedAt", generatedAt)
            .put(
                "snapshot",
                JSONObject().put("schemaVersion", 1).put("generatedAt", generatedAt).put("providers", providers),
            )
            .toString()
    }
}
