package com.shadelight.iausage.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.shadelight.iausage.data.Formatters

/** Never lets a refresh failure read as "no data": a stale snapshot still
 * shows its age and the last good values stay on screen underneath. */
@Composable
fun ConnectionStatus(generatedAt: String, stale: Boolean, modifier: Modifier = Modifier) {
    Column(modifier, verticalArrangement = Arrangement.spacedBy(2.dp)) {
        if (stale) {
            Text("⚠ Datos antiguos", style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.error)
            Text("Última actualización ${Formatters.formatAge(generatedAt)}", style = MaterialTheme.typography.bodySmall)
        } else {
            Text("● PC conectado", style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)
            Text("Actualizado ${Formatters.formatAge(generatedAt)}", style = MaterialTheme.typography.bodySmall)
        }
    }
}
