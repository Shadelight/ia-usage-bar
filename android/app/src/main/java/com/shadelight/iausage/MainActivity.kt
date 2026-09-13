package com.shadelight.iausage

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import com.google.mlkit.vision.codescanner.GmsBarcodeScanning
import com.google.mlkit.vision.codescanner.GmsBarcodeScannerOptions
import com.google.mlkit.vision.barcode.common.Barcode
import com.shadelight.iausage.data.PairingInfo
import com.shadelight.iausage.data.PairingUri
import com.shadelight.iausage.data.ProviderUsage
import com.shadelight.iausage.data.SyncPayload
import com.shadelight.iausage.data.UsageSyncRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class MainActivity : ComponentActivity() {
    private val viewModel by lazy {
        ViewModelProvider(this, UsageViewModel.Factory(applicationContext))[UsageViewModel::class.java]
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent { MaterialTheme { Surface(Modifier.fillMaxSize()) { UsageApp(viewModel, ::scanQr) } } }
    }

    private fun scanQr() {
        val options = GmsBarcodeScannerOptions.Builder().setBarcodeFormats(Barcode.FORMAT_QR_CODE).enableAutoZoom().build()
        GmsBarcodeScanning.getClient(this, options).startScan()
            .addOnSuccessListener { barcode -> barcode.rawValue?.let(viewModel::setPairingUri) }
            .addOnFailureListener { error -> viewModel.setError("No se pudo leer el QR: ${error.message ?: "inténtalo otra vez"}") }
    }
}

data class UsageUiState(
    val pairing: PairingInfo? = null,
    val payload: SyncPayload? = null,
    val loading: Boolean = false,
    val error: String? = null,
)

class UsageViewModel(private val repository: UsageSyncRepository, private val appContext: android.content.Context) : ViewModel() {
    private val _state = MutableStateFlow(UsageUiState(pairing = repository.pairing(), payload = repository.cachedPayload()))
    val state = _state.asStateFlow()

    fun setPairingUri(raw: String) {
        _state.value = _state.value.copy(pairing = runCatching { PairingUri.parse(raw) }.getOrElse { setError(it.message ?: "QR inválido"); return }, error = null)
    }
    fun setError(message: String) { _state.value = _state.value.copy(error = message, loading = false) }
    fun dismissError() { _state.value = _state.value.copy(error = null) }
    fun clearPendingPair() { _state.value = _state.value.copy(pairing = null, error = null) }

    fun pair(passphrase: String) = launch {
        val pairing = _state.value.pairing ?: return@launch
        require(passphrase.isNotBlank()) { "Introduce la passphrase." }
        val payload = repository.pair(pairing, passphrase.toCharArray())
        RefreshWorker.schedule(appContext)
        _state.value = UsageUiState(payload = payload)
    }
    fun refresh() = launch { _state.value = _state.value.copy(payload = repository.refresh(), error = null) }
    fun disconnect() {
        repository.disconnect(); RefreshWorker.cancel(appContext); _state.value = UsageUiState()
    }

    private fun launch(block: suspend () -> Unit) = viewModelScope.launch {
        _state.value = _state.value.copy(loading = true, error = null)
        runCatching { withContext(Dispatchers.IO) { block() } }.onFailure { setError(it.message ?: "No se pudo completar la sincronización.") }
        if (_state.value.error == null) _state.value = _state.value.copy(loading = false)
    }

    class Factory(context: android.content.Context) : ViewModelProvider.Factory {
        private val appContext = context.applicationContext
        @Suppress("UNCHECKED_CAST") override fun <T : ViewModel> create(modelClass: Class<T>): T = UsageViewModel(UsageSyncRepository(appContext), appContext) as T
    }
}

@Composable
private fun UsageApp(viewModel: UsageViewModel, scanQr: () -> Unit) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    // ponytail: without remember, every recomposition (i.e. every keystroke,
    // since typing changes state) re-ran this function body and reset these
    // back to "" — the fields looked like they refused input entirely.
    var manualUri by remember { mutableStateOf("") }
    var passphrase by remember(state.pairing) { mutableStateOf("") }
    var passphraseVisible by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxSize().padding(24.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        Text("IA Usage", style = MaterialTheme.typography.headlineMedium)
        state.error?.let { Card { Row(Modifier.padding(12.dp)) { Text(it, Modifier.weight(1f)); TextButton(viewModel::dismissError) { Text("Cerrar") } } } }
        when {
            state.payload != null -> Dashboard(state.payload!!, state.loading, viewModel::refresh, viewModel::disconnect)
            state.pairing == null -> {
                Text("Vincula tu computadora para consultar tus límites de IA.", style = MaterialTheme.typography.bodyLarge)
                Button(onClick = scanQr, enabled = !state.loading, modifier = Modifier.fillMaxWidth()) { Text("Escanear QR") }
                Text("o introduce la dirección de pareo", style = MaterialTheme.typography.labelLarge)
                OutlinedTextField(manualUri, { manualUri = it }, Modifier.fillMaxWidth(), label = { Text("iausage://pair?…") }, singleLine = false)
                Button(onClick = { viewModel.setPairingUri(manualUri) }, enabled = manualUri.isNotBlank() && !state.loading, modifier = Modifier.fillMaxWidth()) { Text("Continuar") }
            }
            else -> {
                val pairing = state.pairing!!
                Text("PC encontrado", style = MaterialTheme.typography.titleLarge)
                Card(Modifier.fillMaxWidth()) { Column(Modifier.padding(16.dp)) {
                    Text("IA Usage Bar", style = MaterialTheme.typography.titleMedium)
                    Text("${pairing.host}:${pairing.port}")
                    Spacer(Modifier.padding(4.dp))
                    Text("Fingerprint: ${pairing.fingerprint}", style = MaterialTheme.typography.labelLarge)
                } }
                Text("Comprueba que el fingerprint coincide con el mostrado en el PC.")
                OutlinedTextField(
                    passphrase, { passphrase = it }, Modifier.fillMaxWidth(),
                    label = { Text("Frase secreta") },
                    visualTransformation = if (passphraseVisible) VisualTransformation.None else PasswordVisualTransformation(),
                    singleLine = true,
                    trailingIcon = { TextButton(onClick = { passphraseVisible = !passphraseVisible }) { Text(if (passphraseVisible) "Ocultar" else "Mostrar") } },
                )
                Button(onClick = { viewModel.pair(passphrase) }, enabled = passphrase.isNotBlank() && !state.loading, modifier = Modifier.fillMaxWidth()) { Text(if (state.loading) "Vinculando…" else "Vincular") }
                TextButton(onClick = viewModel::clearPendingPair) { Text("Usar otro QR") }
            }
        }
    }
}

@Composable
private fun Dashboard(payload: SyncPayload, loading: Boolean, refresh: () -> Unit, disconnect: () -> Unit) {
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
