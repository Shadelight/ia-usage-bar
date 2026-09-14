package com.shadelight.iausage.ui.components

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.shadelight.iausage.data.ReleaseUpdate
import com.shadelight.iausage.data.UpdateState
import com.shadelight.iausage.ui.UpdateViewModel

/** Lanza el instalador del sistema o, sin permiso, la pantalla que lo otorga. */
fun launchUpdateInstall(context: Context, viewModel: UpdateViewModel, release: ReleaseUpdate, fileName: String) {
    if (!viewModel.canInstallPackages()) {
        context.startActivity(viewModel.unknownSourcesIntent())
        return
    }
    // El APK pudo desaparecer (limpieza de caché): ante la duda se vuelve a
    // descargar y reverificar, nunca se instala sin checksum OK.
    val intent = viewModel.installerIntent(fileName) ?: run {
        viewModel.download(release)
        return
    }
    context.startActivity(intent)
}

fun openReleasePage(context: Context, release: ReleaseUpdate) {
    runCatching {
        context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(release.releaseUrl)))
    }
}

/**
 * Aviso global de actualización: vive en el Scaffold raíz (fuera del `when`
 * de pairing) para que un usuario sin PC vinculado también se entere. Solo
 * cubre los estados accionables; `Failed`/`UpToDate` se muestran en
 * Acerca de, donde nació el chequeo manual.
 */
@Composable
fun UpdateBanner(
    state: UpdateState,
    dismissedTag: String?,
    viewModel: UpdateViewModel,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    when (state) {
        is UpdateState.Available -> {
            if (state.release.tag == dismissedTag) return
            Row(modifier.fillMaxWidth().padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                Text(
                    "Nueva versión disponible (${state.release.version})",
                    modifier = Modifier.weight(1f),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.primary,
                )
                TextButton(onClick = { viewModel.download(state.release) }) { Text("Actualizar") }
                TextButton(onClick = { viewModel.dismiss(state.release.tag) }) { Text("Ahora no") }
            }
        }
        is UpdateState.Downloading -> {
            Column(modifier.fillMaxWidth().padding(vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(
                    "Descargando ${state.release.version}…",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.primary,
                )
                if (state.progress != null) {
                    LinearProgressIndicator(progress = { state.progress }, modifier = Modifier.fillMaxWidth())
                } else {
                    LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
                }
            }
        }
        is UpdateState.Verifying -> {
            Column(modifier.fillMaxWidth().padding(vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text("Verificando actualización…", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.primary)
                LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
            }
        }
        is UpdateState.ReadyToInstall -> {
            Row(modifier.fillMaxWidth().padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                Text(
                    "Actualización ${state.release.version} lista",
                    modifier = Modifier.weight(1f),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.primary,
                )
                TextButton(onClick = { launchUpdateInstall(context, viewModel, state.release, state.fileName) }) { Text("Instalar") }
            }
        }
        else -> {}
    }
}
