package com.shadelight.iausage

import android.content.Context
import androidx.glance.appwidget.updateAll
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import com.shadelight.iausage.data.UsageSyncRepository
import com.shadelight.iausage.widget.UsageWidget
import java.util.concurrent.TimeUnit

class RefreshWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result = try {
        UsageSyncRepository(applicationContext).refresh()
        UsageWidget().updateAll(applicationContext)
        Result.success()
    } catch (_: Exception) {
        Result.retry()
    }

    companion object {
        fun schedule(context: Context) {
            val request = PeriodicWorkRequestBuilder<RefreshWorker>(15, TimeUnit.MINUTES)
                .setConstraints(androidx.work.Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
                .build()
            WorkManager.getInstance(context).enqueueUniquePeriodicWork("iausage-refresh", ExistingPeriodicWorkPolicy.UPDATE, request)
        }
        fun cancel(context: Context) = WorkManager.getInstance(context).cancelUniqueWork("iausage-refresh")
    }
}
