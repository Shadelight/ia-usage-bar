package com.shadelight.iausage.widget

import android.content.Context
import android.content.Intent
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.glance.GlanceModifier
import androidx.glance.action.clickable
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.action.actionStartActivity
import androidx.glance.appwidget.provideContent
import androidx.glance.background
import androidx.glance.layout.Column
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.padding
import androidx.glance.text.Text
import androidx.glance.text.TextStyle
import androidx.glance.unit.ColorProvider
import com.shadelight.iausage.MainActivity
import com.shadelight.iausage.data.SecureStateStore
import com.shadelight.iausage.data.SyncPayload
import org.json.JSONObject
import androidx.glance.appwidget.cornerRadius

class UsageWidget : GlanceAppWidget() {
    override suspend fun provideGlance(context: Context, id: androidx.glance.GlanceId) {
        provideContent { WidgetContent() }
    }

    @Composable private fun WidgetContent() {
        val context = LocalContext.current
        // ponytail: a widget host (notably Samsung One UI) shows a permanent
        // "can't load widget" placeholder if provideGlance ever throws, so
        // any failure here must degrade to an error row instead of crashing.
        val payload = runCatching {
            SecureStateStore(context).payload()?.let { runCatching { SyncPayload.fromJson(JSONObject(it)) }.getOrNull() }
        }.getOrNull()
        Column(
            modifier = GlanceModifier.fillMaxSize().background(Color(0xFF15233B)).cornerRadius(16.dp).padding(14.dp)
                .clickable(actionStartActivity(Intent(context, MainActivity::class.java))),
        ) {
            Text("IA Usage", style = TextStyle(color = ColorProvider(Color(0xFF70E0B6))))
            if (payload == null) Text("Abre la app para vincular un PC", style = TextStyle(color = ColorProvider(Color.White)))
            else {
                payload.snapshot.providers.filter { it.enabled }.take(3).forEach { provider ->
                    val quota = provider.quotas.firstOrNull()
                    Text("${provider.name}  ${quota?.usedPercent?.toInt()?.let { "$it%" } ?: "—"}", style = TextStyle(color = ColorProvider(Color.White)))
                }
                Text(if (payload.snapshot.providers.any { it.stale }) "Datos antiguos" else "Actualizado", style = TextStyle(color = ColorProvider(Color(0xFFC5D3E6))))
            }
        }
    }
}

class UsageWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = UsageWidget()
}
