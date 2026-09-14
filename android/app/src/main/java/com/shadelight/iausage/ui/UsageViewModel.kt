package com.shadelight.iausage.ui

import android.content.Context
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.shadelight.iausage.RefreshWorker
import com.shadelight.iausage.alerts.MonitorService
import com.shadelight.iausage.alerts.MonitorSettings
import com.shadelight.iausage.data.AppPreferences
import com.shadelight.iausage.data.PairingInfo
import com.shadelight.iausage.data.PairingUri
import com.shadelight.iausage.data.SyncPayload
import com.shadelight.iausage.data.ThemeMode
import com.shadelight.iausage.data.UsageSyncRepository
import com.shadelight.iausage.data.UserPreferencesRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

data class UsageUiState(
    val pairing: PairingInfo? = null,
    val payload: SyncPayload? = null,
    /** Blocks primary actions (initial pair, first load). */
    val loading: Boolean = false,
    /** A refresh in flight while data is already visible: never hide the
     * existing values for this, just show a small indicator. */
    val refreshing: Boolean = false,
    val error: String? = null,
)

class UsageViewModel(
    private val repository: UsageSyncRepository,
    private val preferencesRepository: UserPreferencesRepository,
    private val appContext: Context,
) : ViewModel() {
    private val _state = MutableStateFlow(UsageUiState(pairing = repository.pairing(), payload = repository.cachedPayload()))
    val state = _state.asStateFlow()

    val preferences: StateFlow<AppPreferences> = preferencesRepository.preferences
        .stateIn(viewModelScope, SharingStarted.Eagerly, AppPreferences())

    private val _monitorEnabled = MutableStateFlow(MonitorSettings.isEnabled(appContext))
    val monitorEnabled = _monitorEnabled.asStateFlow()

    fun setMonitorEnabled(enabled: Boolean) {
        MonitorSettings.setEnabled(appContext, enabled)
        _monitorEnabled.value = enabled
    }

    fun setPairingUri(raw: String) {
        _state.value = _state.value.copy(
            pairing = runCatching { PairingUri.parse(raw) }.getOrElse { setError(it.message ?: "QR inválido"); return },
            error = null,
        )
    }
    fun setError(message: String) { _state.value = _state.value.copy(error = message, loading = false, refreshing = false) }
    fun dismissError() { _state.value = _state.value.copy(error = null) }
    fun clearPendingPair() { _state.value = _state.value.copy(pairing = null, error = null) }

    fun pair(passphrase: String) = launch(blocking = true) {
        val pairing = _state.value.pairing ?: return@launch
        val payload = if (pairing.secret != null) {
            repository.pairV2(pairing)
        } else {
            require(passphrase.isNotBlank()) { "Introduce la frase secreta." }
            repository.pair(pairing, passphrase.toCharArray())
        }
        RefreshWorker.schedule(appContext)
        if (MonitorSettings.isEnabled(appContext)) MonitorService.start(appContext)
        // Keep `pairing` in state (not just the encrypted store) — DeviceScreen
        // needs the host/port/verification code right after pairing succeeds,
        // not just on the next cold start.
        _state.value = UsageUiState(payload = payload, pairing = pairing)
    }

    /** Used by both pull-to-refresh and the toolbar action: the cached
     * payload stays visible the whole time, `refreshing` just toggles a
     * small indicator instead of a full-screen spinner. */
    fun refresh() = launch(blocking = false) {
        _state.value = _state.value.copy(payload = repository.refresh(), error = null)
    }

    fun disconnect() {
        // The alerts preference survives a disconnect; only the polling stops
        // until a PC is paired again.
        repository.disconnect(); RefreshWorker.cancel(appContext); MonitorService.stop(appContext); _state.value = UsageUiState()
    }

    /** "Limpiar datos locales" (Settings → Datos): drops only the cached
     * snapshot. The PC stays paired — this is not "Desvincular". */
    fun clearLocalData() = launch(blocking = true) {
        repository.clearCachedPayload()
        val payload = repository.refresh()
        _state.value = _state.value.copy(payload = payload)
    }

    fun setTheme(theme: ThemeMode) = viewModelScope.launch { preferencesRepository.setTheme(theme) }
    fun setUsedMode(usedMode: Boolean) = viewModelScope.launch { preferencesRepository.setUsedMode(usedMode) }
    fun setVisibleProviderIds(ids: Set<String>?) = viewModelScope.launch { preferencesRepository.setVisibleProviderIds(ids) }

    private fun launch(blocking: Boolean, block: suspend () -> Unit) = viewModelScope.launch {
        _state.value = if (blocking) _state.value.copy(loading = true, error = null) else _state.value.copy(refreshing = true, error = null)
        runCatching { withContext(Dispatchers.IO) { block() } }.onFailure { setError(it.message ?: "No se pudo completar la sincronización.") }
        if (_state.value.error == null) _state.value = _state.value.copy(loading = false, refreshing = false)
    }

    class Factory(context: Context) : ViewModelProvider.Factory {
        private val appContext = context.applicationContext
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T =
            UsageViewModel(UsageSyncRepository(appContext), UserPreferencesRepository(appContext), appContext) as T
    }
}
