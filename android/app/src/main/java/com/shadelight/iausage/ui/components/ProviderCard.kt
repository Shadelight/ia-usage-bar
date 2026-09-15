package com.shadelight.iausage.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Card
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.shadelight.iausage.data.Formatters
import com.shadelight.iausage.data.ProviderUsage
import com.shadelight.iausage.data.QuotaGroup
import com.shadelight.iausage.data.providerVisual

private val StatusConnected = Color(0xFF4ADE80)
private val StatusPartial = Color(0xFFFBBF24)
private val StatusError = Color(0xFFF87171)
private val StatusIdle = Color(0xFF9CA3AF)

private fun statusColor(provider: ProviderUsage): Color {
    val hasData = provider.quotas.any { Formatters.isSummaryVisible(it.visible) && it.usedPercent != null }
    if (!hasData) return StatusIdle
    if (provider.stale) return StatusPartial
    return when (provider.connectionStatus) {
        "needs_auth", "error", "auth_error" -> StatusError
        "connected", null -> when (provider.availability) {
            "partial_limited" -> StatusPartial
            "blocked" -> StatusError
            else -> StatusConnected
        }
        else -> StatusError
    }
}

/** Collapsed: one row per quota group (most restrictive window). Expanded:
 * windows inside each group, plus connection/source/updated details. */
@Composable
fun ProviderCard(
    provider: ProviderUsage,
    usedMode: Boolean,
    expanded: Boolean,
    onToggleExpanded: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val visual = providerVisual(provider.id, provider.name)
    val groups = Formatters.groupsFromQuotas(provider.quotas)
    Card(
        modifier
            .fillMaxWidth()
            .clickable(onClickLabel = if (expanded) "Contraer" else "Expandir") { onToggleExpanded() }
            .semantics { role = Role.Button },
    ) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                if (visual.icon != null) {
                    Image(painterResource(visual.icon), contentDescription = null, modifier = Modifier.size(22.dp))
                } else {
                    Text(visual.shortCode, style = MaterialTheme.typography.labelLarge)
                }
                Text(provider.name, style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
                provider.plan?.takeIf { it.isNotBlank() }?.let { plan ->
                    Text(plan, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Box(
                    Modifier
                        .size(10.dp)
                        .background(statusColor(provider), CircleShape)
                        .semantics { contentDescription = "" },
                )
            }
            if (groups.isEmpty()) {
                Text("Sin datos todavía", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            } else if (!expanded) {
                groups.forEach { group -> GroupSummaryRow(group) }
                if (groups.size > 1) {
                    Text(
                        "${groups.size} grupos · tocar para ver cuotas",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            AnimatedVisibility(visible = expanded, enter = fadeIn() + expandVertically(), exit = fadeOut() + shrinkVertically()) {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    HorizontalDivider()
                    groups.forEach { group ->
                        val named = groups.size > 1 || group.models.isNotEmpty()
                        if (named) {
                            val exhausted = Formatters.groupSummary(group)?.first == 100
                            Text(
                                if (exhausted) "${group.label} ⚠" else group.label,
                                style = MaterialTheme.typography.titleSmall,
                            )
                            if (group.models.isNotEmpty()) {
                                Text(
                                    group.models.joinToString(" · "),
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                        group.quotas.forEach { QuotaProgress(it, usedMode) }
                    }
                    provider.credits?.let { credits ->
                        Text(
                            "Créditos: ${credits.remaining.toLong()}" + if (credits.stale) " · datos antiguos" else "",
                            style = MaterialTheme.typography.bodySmall,
                        )
                        credits.resetsAvailable?.let { resets ->
                            Text(
                                "Restablecimientos disponibles: $resets" + if (credits.resetsStale) " · datos antiguos" else "",
                                style = MaterialTheme.typography.bodySmall,
                            )
                        }
                    }
                    provider.connectionStatus?.takeUnless { it == "connected" }?.let { status ->
                        Text(
                            "Estado: $status" + (provider.connectionReason?.let { " ($it)" } ?: ""),
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                    Text(
                        "Actualizado: ${provider.updatedAt ?: "sin datos"}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

@Composable
private fun GroupSummaryRow(group: QuotaGroup) {
    val summary = Formatters.groupSummary(group)
    val exhausted = summary?.first == 100
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
        Text(
            if (exhausted) "${group.label} ⚠" else group.label,
            style = MaterialTheme.typography.bodyMedium,
            modifier = Modifier.weight(1f).padding(end = 8.dp),
        )
        Text(
            Formatters.percentPair(Formatters.groupUsedExact(group)),
            style = MaterialTheme.typography.bodyMedium,
        )
    }
}
