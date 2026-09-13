package com.shadelight.iausage.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.shadelight.iausage.data.ProviderUsage
import com.shadelight.iausage.data.SyncPayload

@Composable
fun DashboardScreen(payload: SyncPayload, loading: Boolean, refresh: () -> Unit, disconnect: () -> Unit) {
    val stale = payload.snapshot.providers.any { it.stale || it.quotas.any { quota -> quota.stale } }
    Text(if (stale) "Datos antiguos · ${payload.generatedAt}" else "PC conectado · ${payload.generatedAt}", style = MaterialTheme.typography.bodyMedium)
    Row(Modifier.fillMaxWidth()) {
        Button(onClick = refresh, enabled = !loading, modifier = Modifier.weight(1f)) { Text(if (loading) "Actualizando…" else "Actualizar") }
        Spacer(Modifier.width(8.dp)); TextButton(onClick = disconnect) { Text("Desvincular") }
    }
    LazyColumn(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        items(payload.snapshot.providers.filter { it.enabled }, key = { it.id }) { ProviderCard(it) }
    }
}

@Composable
private fun ProviderCard(provider: ProviderUsage) {
    var expanded by remember { mutableStateOf(false) }
    Card(Modifier.fillMaxWidth().clickable { expanded = !expanded }) { Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row { Text(provider.name, Modifier.weight(1f), style = MaterialTheme.typography.titleMedium); if (provider.stale) Text("Antiguo", color = MaterialTheme.colorScheme.error) }
        provider.connectionStatus?.takeUnless { it == "connected" }?.let { Text("Estado: $it${provider.connectionReason?.let { reason -> " ($reason)" } ?: ""}") }
        provider.quotas.forEach { quota ->
            Row { Text(quota.label, Modifier.weight(1f)); Text(quota.usedPercent?.let { "${it.toInt()}%" } ?: "—") }
            quota.resetInSeconds?.let { Text("Reinicia ${formatDuration(it)}", style = MaterialTheme.typography.bodySmall) }
        }
        if (expanded) { HorizontalDivider(); Text("Actualizado: ${provider.updatedAt ?: "sin datos"}", style = MaterialTheme.typography.bodySmall) }
    } }
}

private fun formatDuration(seconds: Long): String = when {
    seconds < 60 -> "en menos de 1 min"
    seconds < 3600 -> "en ${seconds / 60} min"
    seconds < 86_400 -> "en ${seconds / 3600} h"
    else -> "en ${seconds / 86_400} d"
}
