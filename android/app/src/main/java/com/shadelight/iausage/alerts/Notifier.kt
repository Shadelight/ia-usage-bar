package com.shadelight.iausage.alerts

import android.Manifest
import android.annotation.SuppressLint
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import com.shadelight.iausage.MainActivity
import com.shadelight.iausage.R
import com.shadelight.iausage.data.Formatters
import com.shadelight.iausage.data.SyncPayload
import com.shadelight.iausage.data.providerShortName

object Notifier {
    const val CHANNEL_ALERTS = "usage_alerts"
    const val CHANNEL_MONITOR = "usage_monitor"
    const val MONITOR_NOTIFICATION_ID = 1001

    fun ensureChannels(context: Context) {
        val manager = context.getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL_ALERTS, "Avisos de uso", NotificationManager.IMPORTANCE_DEFAULT).apply {
                description = "Reinicios, cuotas por agotarse, mejor opción y desconexión del PC"
            },
        )
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL_MONITOR, "Vigilancia en segundo plano", NotificationManager.IMPORTANCE_LOW).apply {
                description = "Notificación fija mientras IA Usage vigila tus cuotas"
                setShowBadge(false)
            },
        )
    }

    fun canPostAlerts(context: Context): Boolean {
        val granted = Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED
        return granted && NotificationManagerCompat.from(context).areNotificationsEnabled()
    }

    @SuppressLint("MissingPermission") // guarded by canPostAlerts
    fun post(context: Context, alert: UsageAlert) {
        if (!canPostAlerts(context)) return
        ensureChannels(context)
        val notification = NotificationCompat.Builder(context, CHANNEL_ALERTS)
            .setSmallIcon(R.drawable.ic_ia_usage_mark_mono)
            .setContentTitle(alert.title)
            .setContentText(alert.body)
            .setContentIntent(openApp(context))
            .setAutoCancel(true)
            .setCategory(NotificationCompat.CATEGORY_STATUS)
            .build()
        NotificationManagerCompat.from(context).notify(alert.key.hashCode(), notification)
    }

    fun monitorNotification(context: Context, payload: SyncPayload?): Notification {
        ensureChannels(context)
        return NotificationCompat.Builder(context, CHANNEL_MONITOR)
            .setSmallIcon(R.drawable.ic_ia_usage_mark_mono)
            .setContentTitle("IA Usage vigila tus cuotas")
            .setContentText(payload?.let(::summary) ?: "Esperando datos del PC…")
            .setContentIntent(openApp(context))
            .setOngoing(true)
            .setSilent(true)
            .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)
            .build()
    }

    @SuppressLint("MissingPermission") // guarded by canPostAlerts
    fun updateMonitor(context: Context, payload: SyncPayload) {
        if (!canPostAlerts(context)) return
        NotificationManagerCompat.from(context).notify(MONITOR_NOTIFICATION_ID, monitorNotification(context, payload))
    }

    /** "Claude 64% · Codex 0% · Cursor 40%" — used percent, up to three. */
    internal fun summary(payload: SyncPayload): String = payload.snapshot.providers
        .filter { it.enabled }
        .mapNotNull { provider ->
            Formatters.primaryQuota(provider, null)?.usedPercent?.let { "${providerShortName(provider.id, provider.name)} ${it.toInt()}%" }
        }
        .take(3)
        .joinToString(" · ")
        .ifEmpty { "Sin cuotas todavía" }

    private fun openApp(context: Context): PendingIntent = PendingIntent.getActivity(
        context,
        0,
        Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )
}
