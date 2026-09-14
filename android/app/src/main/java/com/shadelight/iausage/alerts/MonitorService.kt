package com.shadelight.iausage.alerts

import android.app.Service
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.ServiceCompat
import androidx.core.content.ContextCompat
import androidx.glance.appwidget.updateAll
import com.shadelight.iausage.data.UsageSyncRepository
import com.shadelight.iausage.widget.UsageWidget
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

/** Keeps quotas in sync with the paired PC while the user opted into
 * background alerts. WorkManager's 15-minute floor is too slow to warn before
 * a session runs out, so this polls the LAN server every few minutes.
 * Android requires the ongoing notification for that, and `specialUse` is
 * the only fitting foreground type without a daily cap (`dataSync` is capped
 * at 6 h a day on Android 15). */
class MonitorService : Service() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var loop: Job? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        // startForegroundService() demands startForeground() promptly, even
        // when we are about to stop, or the system kills the process.
        val type = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE
        } else {
            0
        }
        ServiceCompat.startForeground(this, Notifier.MONITOR_NOTIFICATION_ID, Notifier.monitorNotification(this, null), type)
        if (!MonitorSettings.isEnabled(this)) {
            stopSelf()
            return START_NOT_STICKY
        }
        if (loop?.isActive != true) {
            loop = scope.launch {
                while (isActive) {
                    tick()
                    delay(POLL_INTERVAL_MS)
                }
            }
        }
        return START_STICKY
    }

    private suspend fun tick() {
        val repository = UsageSyncRepository(applicationContext)
        if (repository.pairing() == null) return
        runCatching { repository.refresh() }
            .onSuccess { payload ->
                AlertPipeline.onRefreshed(applicationContext, payload)
                Notifier.updateMonitor(applicationContext, payload)
                runCatching { UsageWidget().updateAll(applicationContext) }
            }
            .onFailure { AlertPipeline.onRefreshFailed(applicationContext) }
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }

    companion object {
        const val POLL_INTERVAL_MS = 3 * 60 * 1000L

        /** Only from a foreground context (the app UI) or the boot receiver:
         * Android 12+ refuses to start a foreground service from background. */
        fun start(context: Context) {
            runCatching { ContextCompat.startForegroundService(context, Intent(context, MonitorService::class.java)) }
        }

        fun stop(context: Context) {
            context.stopService(Intent(context, MonitorService::class.java))
        }
    }
}

/** Restores the monitor after a reboot or an app update when it was on. */
class MonitorBootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val action = intent.action ?: return
        if (action != Intent.ACTION_BOOT_COMPLETED && action != Intent.ACTION_MY_PACKAGE_REPLACED) return
        if (MonitorSettings.isEnabled(context)) MonitorService.start(context)
    }
}
