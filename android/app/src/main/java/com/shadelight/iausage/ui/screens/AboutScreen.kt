package com.shadelight.iausage.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.shadelight.iausage.BuildConfig

@Composable
fun AboutScreen(modifier: Modifier = Modifier) {
    Column(modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text("IA Usage", style = MaterialTheme.typography.headlineSmall)
        Text("Versión ${BuildConfig.VERSION_NAME}", style = MaterialTheme.typography.bodyMedium)
        Text(
            "Visor de límites, cuotas y uso de IA sincronizado de forma segura desde IA Usage Desktop.",
            style = MaterialTheme.typography.bodyMedium,
        )
        Text("Creado por Alberth Salazar.", style = MaterialTheme.typography.bodyMedium)
    }
}
