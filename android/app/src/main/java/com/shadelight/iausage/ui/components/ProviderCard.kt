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
import com.shadelight.iausage.data.ProviderUsage
import com.shadelight.iausage.data.providerVisual

private val StatusConnected = Color(0xFF4ADE80)
private val StatusStale = Color(0xFFFBBF24)
private val StatusError = Color(0xFFF87171)

private fun statusColor(provider: ProviderUsage): Color = when {
    provider.stale -> StatusStale
    provider.connectionStatus == "connected" -> StatusConnected
    provider.connectionStatus == null -> StatusConnected
    else -> StatusError
}

/** Collapsed: icon + name + primary quota + status dot. Expanded: every
 * quota with progress, plus connection/source/updated details. Only the
 * provider accent touches the icon/dot/progress — never the whole card, so
 * ten enabled providers don't turn the dashboard into a color wheel. */
@Composable
fun ProviderCard(
    provider: ProviderUsage,
    usedMode: Boolean,
    expanded: Boolean,
    onToggleExpanded: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val visual = providerVisual(provider.id, provider.name)
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
                Box(
                    Modifier
                        .size(10.dp)
                        .background(statusColor(provider), CircleShape)
                        .semantics { contentDescription = "" },
                )
            }
            provider.quotas.firstOrNull()?.let { primary ->
                QuotaProgress(primary, usedMode)
            } ?: Text("Sin datos todavía", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)

            AnimatedVisibility(visible = expanded, enter = fadeIn() + expandVertically(), exit = fadeOut() + shrinkVertically()) {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    HorizontalDivider()
                    provider.quotas.drop(1).forEach { QuotaProgress(it, usedMode) }
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
