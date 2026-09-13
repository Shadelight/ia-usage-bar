package com.shadelight.iausage

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Surface
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.codescanner.GmsBarcodeScanning
import com.google.mlkit.vision.codescanner.GmsBarcodeScannerOptions
import com.shadelight.iausage.ui.UsageApp
import com.shadelight.iausage.ui.UsageViewModel
import com.shadelight.iausage.ui.resolveDarkTheme
import com.shadelight.iausage.ui.theme.IaUsageTheme
import java.net.URI

/** Keep the exported activity narrow: only the IA Usage pairing URI is
 * handed to the parser/ViewModel. Everything else is ignored. */
internal fun acceptedPairingUri(raw: String?): String? {
    val value = raw?.trim()?.takeIf { it.isNotEmpty() } ?: return null
    val uri = runCatching { URI(value) }.getOrNull() ?: return null
    return value.takeIf { uri.scheme == "iausage" && uri.host == "pair" }
}

/** Kept thin on purpose: activity setup + the QR scanner launch, nothing
 * else. State, navigation and screens all live under ui/. */
class MainActivity : ComponentActivity() {
    private val viewModel by lazy {
        ViewModelProvider(this, UsageViewModel.Factory(applicationContext))[UsageViewModel::class.java]
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        handlePairingIntent(intent)
        enableEdgeToEdge()
        setContent {
            val preferences by viewModel.preferences.collectAsStateWithLifecycle()
            IaUsageTheme(darkTheme = resolveDarkTheme(preferences.theme)) {
                Surface(Modifier.fillMaxSize()) { UsageApp(viewModel, ::scanQr) }
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handlePairingIntent(intent)
    }

    private fun handlePairingIntent(intent: Intent?) {
        acceptedPairingUri(intent?.dataString)?.let(viewModel::setPairingUri)
    }

    private fun scanQr() {
        val options = GmsBarcodeScannerOptions.Builder().setBarcodeFormats(Barcode.FORMAT_QR_CODE).enableAutoZoom().build()
        GmsBarcodeScanning.getClient(this, options).startScan()
            .addOnSuccessListener { barcode -> barcode.rawValue?.let(viewModel::setPairingUri) }
            .addOnFailureListener { error -> viewModel.setError("No se pudo leer el QR: ${error.message ?: "inténtalo otra vez"}") }
    }
}
