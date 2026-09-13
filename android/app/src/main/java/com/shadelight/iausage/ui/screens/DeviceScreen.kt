package com.shadelight.iausage.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
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
import com.shadelight.iausage.data.AUTO_REFRESH_MINUTES
import com.shadelight.iausage.data.Formatters
import com.shadelight.iausage.data.PairingInfo
import com.shadelight.iausage.data.SyncPayload

@Composable
fun DeviceScreen(
    pairing: PairingInfo,
    payload: SyncPayload?,
    refreshing: Boolean,
    onRefreshNow: () -> Unit,
    onDisconnect: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var confirmingDisconnect by remember { mutableStateOf(false) }

    Column(modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text("Dispositivo vinculado", style = MaterialTheme.typography.titleLarge)
        Card(Modifier.fillMaxWidth()) {
            Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("IA Usage Desktop", style = MaterialTheme.typography.titleMedium)
                Text("${pairing.host}:${pairing.port}", style = MaterialTheme.typography.bodyMedium)
                Text("Código de verificación", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(pairing.fingerprint, style = MaterialTheme.typography.titleMedium)
            }
        }
        Card(Modifier.fillMaxWidth()) {
            Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                InfoRow("Última sincronización", payload?.let { Formatters.formatAge(it.generatedAt) } ?: "—")
                InfoRow("Actualización automática", "Cada $AUTO_REFRESH_MINUTES min")
            }
        }
        Button(onClick = onRefreshNow, enabled = !refreshing, modifier = Modifier.fillMaxWidth()) {
            Text(if (refreshing) "Actualizando…" else "Actualizar ahora")
        }
        Text(
            "Seguridad: los datos se cifran antes de salir del PC.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        TextButton(onClick = { confirmingDisconnect = true }) {
            Text("Desvincular PC", color = MaterialTheme.colorScheme.error)
        }
    }

    if (confirmingDisconnect) {
        AlertDialog(
            onDismissRequest = { confirmingDisconnect = false },
            title = { Text("¿Desvincular este PC?") },
            text = { Text("Vas a dejar de recibir datos de uso hasta que vuelvas a vincularlo.") },
            confirmButton = {
                TextButton(onClick = { confirmingDisconnect = false; onDisconnect() }) {
                    Text("Desvincular", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = { TextButton(onClick = { confirmingDisconnect = false }) { Text("Cancelar") } },
        )
    }
}

@Composable
private fun InfoRow(label: String, value: String) {
    Text(label, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    Text(value, style = MaterialTheme.typography.bodyMedium)
}
