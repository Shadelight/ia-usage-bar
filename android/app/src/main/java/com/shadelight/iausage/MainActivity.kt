package com.shadelight.iausage

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
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
import com.shadelight.iausage.data.SyncPayload
import com.shadelight.iausage.data.UsageSyncRepository
import com.shadelight.iausage.ui.screens.DashboardScreen
import com.shadelight.iausage.ui.screens.NoPairingScreen
import com.shadelight.iausage.ui.screens.PendingPairingScreen
import com.shadelight.iausage.ui.theme.IaUsageTheme
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
        setContent { IaUsageTheme { Surface(Modifier.fillMaxSize()) { UsageApp(viewModel, ::scanQr) } } }
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
    Column(Modifier.fillMaxSize().padding(24.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        Text("IA Usage", style = MaterialTheme.typography.headlineMedium)
        state.error?.let { Card { Row(Modifier.padding(12.dp)) { Text(it, Modifier.weight(1f)); TextButton(viewModel::dismissError) { Text("Cerrar") } } } }
        when {
            state.payload != null -> DashboardScreen(state.payload!!, state.loading, viewModel::refresh, viewModel::disconnect)
            state.pairing == null -> NoPairingScreen(state.loading, scanQr, viewModel::setPairingUri)
            else -> PendingPairingScreen(state.pairing!!, state.loading, viewModel::pair, viewModel::clearPendingPair)
        }
    }
}
