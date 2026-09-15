package com.shadelight.iausage.ui.components

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.shadelight.iausage.data.Formatters
import com.shadelight.iausage.data.UsageQuota

/** One quota row: label, used·available, a progress bar, and its reset —
 * the one place this is drawn, so Dashboard and Device screens agree. */
@Composable
fun QuotaProgress(quota: UsageQuota, usedMode: Boolean, modifier: Modifier = Modifier) {
    val summary = quota.usedPercent?.let { Formatters.summaryPercents(it) }
    val fraction = ((summary?.first ?: 0) / 100f).coerceIn(0f, 1f)
    val animatedFraction by animateFloatAsState(targetValue = fraction, animationSpec = tween(220), label = "quota-progress")
    val pair = when {
        summary == null -> "—"
        usedMode -> "${summary.first}% usado · ${summary.second}% restante"
        else -> "${summary.second}% restante · ${summary.first}% usado"
    }
    Column(modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(Formatters.windowLabel(quota), style = MaterialTheme.typography.bodyMedium)
            Text(pair, style = MaterialTheme.typography.bodyMedium)
        }
        LinearProgressIndicator(
            progress = { animatedFraction },
            modifier = Modifier
                .fillMaxWidth()
                .semantics { contentDescription = "${Formatters.windowLabel(quota)}: ${summary?.first ?: 0} por ciento usado" },
            trackColor = MaterialTheme.colorScheme.surfaceVariant,
        )
        Formatters.formatResetIn(Formatters.liveResetSeconds(quota))?.let {
            Text(it, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}
