package com.shadelight.iausage.ui

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.content.FileProvider
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.shadelight.iausage.BuildConfig
import com.shadelight.iausage.data.GithubUpdateRepository
import com.shadelight.iausage.data.ReleaseUpdate
import com.shadelight.iausage.data.UpdateState
import com.shadelight.iausage.data.UserPreferencesRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File

/** Cooldown del chequeo automático: 6 horas. El manual lo ignora. */
const val AUTO_UPDATE_CHECK_COOLDOWN_MS = 6 * 60 * 60 * 1000L

fun fileProviderAuthority(context: Context): String = "${context.packageName}.fileprovider"

/**
 * Orquesta la auto-actualización. Filosofía heredada de Windows: ningún fallo
 * del updater destruye estado (pairing, datos, pantalla actual); los errores
 * van a `Failed` o se silencian en el chequeo automático.
 */
class UpdateViewModel(
    private val repository: GithubUpdateRepository,
    private val preferencesRepository: UserPreferencesRepository,
    private val appContext: Context,
) : ViewModel() {
    private val _state = MutableStateFlow<UpdateState>(UpdateState.Idle)
    val state = _state.asStateFlow()

    /** Chequeo silencioso al arrancar: respeta cooldown, falla sin mostrar nada. */
    fun checkSilently() {
        if (_state.value != UpdateState.Idle) return
        viewModelScope.launch {
            val prefs = preferencesRepository.preferences.first()
            if (System.currentTimeMillis() - prefs.lastUpdateCheckAt < AUTO_UPDATE_CHECK_COOLDOWN_MS) return@launch
            runCatching { withContext(Dispatchers.IO) { doCheck() } }
            // Fallo silencioso a propósito: sin error visible, sin reintento
            // agresivo. El timestamp solo avanza en checks con respuesta.
        }
    }

    fun checkManually() {
        if (_state.value == UpdateState.Checking || _state.value is UpdateState.Downloading) return
        _state.value = UpdateState.Checking
        viewModelScope.launch {
            runCatching { withContext(Dispatchers.IO) { doCheck() } }
                .onFailure { _state.value = UpdateState.Failed(it.message ?: "No se pudo consultar la última versión.") }
        }
    }

    private suspend fun doCheck() {
        val result = repository.check(BuildConfig.VERSION_NAME)
        preferencesRepository.setLastUpdateCheckAt(System.currentTimeMillis())
        _state.value = when (result) {
            is GithubUpdateRepository.CheckResult.UpToDate -> UpdateState.UpToDate
            is GithubUpdateRepository.CheckResult.Available -> UpdateState.Available(result.release)
        }
    }

    fun download(release: ReleaseUpdate) {
        if (_state.value is UpdateState.Downloading) return
        _state.value = UpdateState.Downloading(release, null)
        viewModelScope.launch {
            runCatching {
                withContext(Dispatchers.IO) {
                    // Si quedó un APK verificado de un intento anterior, se
                    // reutiliza tras reverificar: nada se instala sin checksum OK.
                    val cached = repository.cachedApk(release.apkName)
                    val part = cached ?: repository.download(release) { progress ->
                        _state.value = UpdateState.Downloading(release, progress)
                    }
                    _state.value = UpdateState.Verifying(release)
                    repository.verify(release, part)
                    repository.promote(part, release.apkName)
                }
            }.onSuccess {
                _state.value = UpdateState.ReadyToInstall(release, release.apkName)
            }.onFailure {
                _state.value = UpdateState.Failed(it.message ?: "No se pudo descargar la actualización.")
            }
        }
    }

    fun dismiss(tag: String) {
        viewModelScope.launch { preferencesRepository.setDismissedUpdateTag(tag) }
        if ((_state.value as? UpdateState.Available)?.release?.tag == tag) {
            _state.value = UpdateState.Idle
        }
    }

    fun canInstallPackages(): Boolean =
        Build.VERSION.SDK_INT < Build.VERSION_CODES.O ||
            appContext.packageManager.canRequestPackageInstalls()

    fun unknownSourcesIntent(): Intent =
        Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES).apply {
            data = Uri.parse("package:${appContext.packageName}")
        }

    /** Intent al instalador del sistema vía FileProvider. Null si el APK ya no está. */
    fun installerIntent(apkName: String): Intent? {
        val apk: File = repository.cachedApk(apkName) ?: return null
        val uri = FileProvider.getUriForFile(appContext, fileProviderAuthority(appContext), apk)
        return Intent(Intent.ACTION_VIEW).apply {
            setDataAndType(uri, "application/vnd.android.package-archive")
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
    }

    class Factory(context: Context) : ViewModelProvider.Factory {
        private val appContext = context.applicationContext
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T =
            UpdateViewModel(GithubUpdateRepository(appContext), UserPreferencesRepository(appContext), appContext) as T
    }
}
