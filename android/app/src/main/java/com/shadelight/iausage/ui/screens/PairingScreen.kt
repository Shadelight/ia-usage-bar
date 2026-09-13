package com.shadelight.iausage.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.dp
import com.shadelight.iausage.data.PairingInfo
import com.shadelight.iausage.ui.components.SecretTextField

@Composable
fun NoPairingScreen(loading: Boolean, scanQr: () -> Unit, onManualUri: (String) -> Unit) {
    // ponytail: remember, not plain mutableStateOf — see PairedScreen below.
    var manualUri by remember { mutableStateOf("") }
    Column(Modifier.fillMaxWidth().padding(24.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        Text("Conecta tu PC", style = MaterialTheme.typography.headlineSmall)
        Text(
            "Obtén el QR desde:\nIA Usage Desktop → Ajustes → Teléfono y sync",
            style = MaterialTheme.typography.bodyMedium,
        )
        Button(onClick = scanQr, enabled = !loading, modifier = Modifier.fillMaxWidth()) { Text("Escanear QR") }
        Text("o introduce la dirección de vinculación", style = MaterialTheme.typography.labelLarge)
        OutlinedTextField(manualUri, { manualUri = it }, Modifier.fillMaxWidth(), label = { Text("iausage://pair?…") }, singleLine = false)
        Button(onClick = { onManualUri(manualUri) }, enabled = manualUri.isNotBlank() && !loading, modifier = Modifier.fillMaxWidth()) { Text("Continuar") }
    }
}

@Composable
fun PendingPairingScreen(pairing: PairingInfo, loading: Boolean, onPair: (String) -> Unit, onUseAnotherQr: () -> Unit) {
    // Keyed on the pairing itself so scanning a different PC's QR starts
    // the passphrase field empty instead of carrying over the old value.
    var passphrase by rememberSaveable(pairing.deviceId) { mutableStateOf("") }
    Column(Modifier.fillMaxWidth().padding(24.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        Text("PC encontrado", style = MaterialTheme.typography.titleLarge)
        Card(Modifier.fillMaxWidth()) {
            Column(Modifier.padding(16.dp)) {
                Text("IA Usage Desktop", style = MaterialTheme.typography.titleMedium)
                Text("${pairing.host}:${pairing.port}")
                Spacer(Modifier.padding(4.dp))
                Text("Código de verificación", style = MaterialTheme.typography.labelLarge)
                Text(pairing.fingerprint, style = MaterialTheme.typography.titleMedium)
            }
        }
        Text("Comprueba que el código coincide con el que ves en el PC.")
        SecretTextField(
            value = passphrase,
            onValueChange = { passphrase = it },
            label = "Frase secreta",
            modifier = Modifier.testTag("pairing-secret"),
            onDone = { if (passphrase.isNotBlank() && !loading) onPair(passphrase) },
        )
        Button(onClick = { onPair(passphrase) }, enabled = passphrase.isNotBlank() && !loading, modifier = Modifier.fillMaxWidth()) {
            Text(if (loading) "Vinculando…" else "Vincular")
        }
        TextButton(onClick = onUseAnotherQr) { Text("Usar otro QR") }
    }
}
