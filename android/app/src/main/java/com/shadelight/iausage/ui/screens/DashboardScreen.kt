package com.shadelight.iausage.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.shadelight.iausage.data.Formatters
import com.shadelight.iausage.data.ProviderUsage
import com.shadelight.iausage.data.SyncPayload
import com.shadelight.iausage.ui.components.ConnectionStatus
import com.shadelight.iausage.ui.components.EmptyState
import com.shadelight.iausage.ui.components.ProviderCard

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DashboardScreen(
    payload: SyncPayload,
    providers: List<ProviderUsage>,
    usedMode: Boolean,
    refreshing: Boolean,
    onRefresh: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val stale = Formatters.isSnapshotStale(providers)
    var expandedId by remember { mutableStateOf<String?>(null) }
    val best = remember(providers) { Formatters.bestAvailableProvider(providers) { null } }

    PullToRefreshBox(isRefreshing = refreshing, onRefresh = onRefresh, modifier = modifier.fillMaxSize()) {
        LazyColumn(Modifier.fillMaxSize().padding(horizontal = 16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            item { ConnectionStatus(payload.generatedAt, stale, Modifier.padding(top = 16.dp)) }

            best?.let { (provider, quota) ->
                item {
                    val available = Formatters.availablePercent(quota.usedPercent)?.toInt() ?: 0
                    Card(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text("Mejor opción ahora", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
                            Text("${provider.name} · $available% disponible", style = MaterialTheme.typography.titleMedium)
                        }
                    }
                }
            }

            if (providers.isEmpty()) {
                item { EmptyState("Sin proveedores para mostrar", detail = "Activa un proveedor en IA Usage Desktop.") }
            } else {
                item { Text("PROVEEDORES", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant) }
                items(providers, key = { it.id }) { provider ->
                    ProviderCard(
                        provider = provider,
                        usedMode = usedMode,
                        expanded = expandedId == provider.id,
                        onToggleExpanded = { expandedId = if (expandedId == provider.id) null else provider.id },
                    )
                }
            }
        }
    }
}
