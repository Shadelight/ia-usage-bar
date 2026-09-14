package com.shadelight.iausage.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.shadelight.iausage.BuildConfig
import com.shadelight.iausage.data.ReleaseUpdate
import com.shadelight.iausage.data.UpdateState
import com.shadelight.iausage.ui.UpdateViewModel
import com.shadelight.iausage.ui.components.launchUpdateInstall
import com.shadelight.iausage.ui.components.openReleasePage

@Composable
fun AboutScreen(
    updateState: UpdateState,
    updateViewModel: UpdateViewModel,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    Column(modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text("IA Usage", style = MaterialTheme.typography.headlineSmall)
        Text("Versión ${BuildConfig.VERSION_NAME}", style = MaterialTheme.typography.bodyMedium)
        Text(
            "Visor de límites, cuotas y uso de IA sincronizado de forma segura desde IA Usage Desktop.",
            style = MaterialTheme.typography.bodyMedium,
        )
        Text("Creado por Alberth Salazar.", style = MaterialTheme.typography.bodyMedium)

        Text("Actualizaciones", style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(top = 12.dp))
        UpdateSection(
            state = updateState,
            onCheck = updateViewModel::checkManually,
            onDownload = updateViewModel::download,
            onInstall = { release, fileName -> launchUpdateInstall(context, updateViewModel, release, fileName) },
            onOpenRelease = { release -> openReleasePage(context, release) },
        )
    }
}

@Composable
private fun UpdateSection(
    state: UpdateState,
    onCheck: () -> Unit,
    onDownload: (ReleaseUpdate) -> Unit,
    onInstall: (ReleaseUpdate, String) -> Unit,
    onOpenRelease: (ReleaseUpdate) -> Unit,
) {
    when (state) {
        is UpdateState.Idle -> {
            Button(onClick = onCheck) { Text("Buscar actualizaciones") }
        }
        is UpdateState.Checking -> {
            Button(onClick = {}, enabled = false) { Text("Buscando…") }
        }
        is UpdateState.UpToDate -> {
            Text("Estás al día (versión ${BuildConfig.VERSION_NAME}).", style = MaterialTheme.typography.bodyMedium)
            TextButton(onClick = onCheck) { Text("Volver a buscar") }
        }
        is UpdateState.Available -> {
            Text("Nueva versión disponible: ${state.release.version}.", style = MaterialTheme.typography.bodyMedium)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                Button(onClick = { onDownload(state.release) }) { Text("Descargar") }
                TextButton(onClick = { onOpenRelease(state.release) }) { Text("Ver cambios") }
            }
        }
        is UpdateState.Downloading -> {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text("Descargando ${state.release.version}…", style = MaterialTheme.typography.bodyMedium)
                if (state.progress != null) {
                    LinearProgressIndicator(progress = { state.progress }, modifier = Modifier.fillMaxWidth())
                } else {
                    LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
                }
            }
        }
        is UpdateState.Verifying -> {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text("Verificando actualización…", style = MaterialTheme.typography.bodyMedium)
                LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
            }
        }
        is UpdateState.ReadyToInstall -> {
            Text("La versión ${state.release.version} está lista para instalar.", style = MaterialTheme.typography.bodyMedium)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                Button(onClick = { onInstall(state.release, state.fileName) }) { Text("Instalar ahora") }
                TextButton(onClick = { onOpenRelease(state.release) }) { Text("Ver cambios") }
            }
        }
        is UpdateState.Failed -> {
            Text(state.reason, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.error)
            TextButton(onClick = onCheck) { Text("Reintentar") }
        }
    }
}
