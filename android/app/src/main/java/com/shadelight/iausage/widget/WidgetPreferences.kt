package com.shadelight.iausage.widget

import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.core.stringSetPreferencesKey

/** Per-instance widget config, stored via Glance's own PreferencesGlanceStateDefinition
 * (androidx.glance.appwidget.state) — the standard Glance mechanism, not a
 * parallel preferences system. Keyed by AppWidgetId under the hood. */
object WidgetPrefsKeys {
    val MODE = stringPreferencesKey("mode") // "auto" or a provider id
    val USED_MODE = booleanPreferencesKey("used_mode")
    val SHOW_RESET = booleanPreferencesKey("show_reset")
    val SHOW_BAR = booleanPreferencesKey("show_bar")
    val SHOW_STATUS = booleanPreferencesKey("show_status")
    val VISIBLE_PROVIDERS = stringSetPreferencesKey("visible_provider_ids") // medium/large only; empty = all
}

const val WIDGET_MODE_AUTO = "auto"

data class WidgetConfig(
    val mode: String = WIDGET_MODE_AUTO,
    val usedMode: Boolean = true,
    val showReset: Boolean = true,
    val showBar: Boolean = true,
    val showStatus: Boolean = true,
    val visibleProviderIds: Set<String> = emptySet(),
)

fun readWidgetConfig(prefs: Preferences): WidgetConfig = WidgetConfig(
    mode = prefs[WidgetPrefsKeys.MODE] ?: WIDGET_MODE_AUTO,
    usedMode = prefs[WidgetPrefsKeys.USED_MODE] ?: true,
    showReset = prefs[WidgetPrefsKeys.SHOW_RESET] ?: true,
    showBar = prefs[WidgetPrefsKeys.SHOW_BAR] ?: true,
    showStatus = prefs[WidgetPrefsKeys.SHOW_STATUS] ?: true,
    visibleProviderIds = prefs[WidgetPrefsKeys.VISIBLE_PROVIDERS] ?: emptySet(),
)
