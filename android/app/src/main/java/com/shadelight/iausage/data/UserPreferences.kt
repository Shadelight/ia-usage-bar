package com.shadelight.iausage.data

import android.content.Context
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.longPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.core.stringSetPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

enum class ThemeMode { SYSTEM, LIGHT, DARK }

/** WorkManager enforces a 15-minute floor on periodic work, so that is the
 * one automatic-refresh interval this app actually offers — showing it as a
 * configurable setting below that floor would just be a lie. */
const val AUTO_REFRESH_MINUTES = 15

data class AppPreferences(
    val theme: ThemeMode = ThemeMode.SYSTEM,
    /** null = show every enabled provider; otherwise the explicit allow-list
     * the user picked. A brand-new provider from the PC shows up by default
     * (null), never silently hidden. */
    val visibleProviderIds: Set<String>? = null,
    val usedMode: Boolean = true,
    /** Tag de release descartado con "Ahora no" (null = ninguno). Un tag
     * distinto vuelve a avisar solo. No es estado de descarga: es preferencia. */
    val dismissedUpdateTag: String? = null,
    /** Epoch-ms del último chequeo automático de actualizaciones. */
    val lastUpdateCheckAt: Long = 0L,
)

/** UI-only preferences: never mixed with the encrypted pairing/passphrase/
 * payload store (SecureStateStore) — these are plain, non-sensitive settings. */
private val Context.dataStore by preferencesDataStore(name = "iausage.preferences")

class UserPreferencesRepository(context: Context) {
    private val store = context.applicationContext.dataStore

    private object Keys {
        val THEME = stringPreferencesKey("theme")
        val VISIBLE_PROVIDERS = stringSetPreferencesKey("visible_provider_ids")
        val HAS_VISIBLE_OVERRIDE = booleanPreferencesKey("has_visible_override")
        val USED_MODE = booleanPreferencesKey("used_mode")
        val DISMISSED_UPDATE_TAG = stringPreferencesKey("dismissed_update_tag")
        val LAST_UPDATE_CHECK_AT = longPreferencesKey("last_update_check_at")
    }

    val preferences: Flow<AppPreferences> = store.data.map { prefs ->
        AppPreferences(
            theme = prefs[Keys.THEME]?.let { runCatching { ThemeMode.valueOf(it) }.getOrNull() } ?: ThemeMode.SYSTEM,
            visibleProviderIds = if (prefs[Keys.HAS_VISIBLE_OVERRIDE] == true) prefs[Keys.VISIBLE_PROVIDERS] ?: emptySet() else null,
            usedMode = prefs[Keys.USED_MODE] ?: true,
            dismissedUpdateTag = prefs[Keys.DISMISSED_UPDATE_TAG],
            lastUpdateCheckAt = prefs[Keys.LAST_UPDATE_CHECK_AT] ?: 0L,
        )
    }

    suspend fun setTheme(theme: ThemeMode) = store.edit { it[Keys.THEME] = theme.name }
    suspend fun setUsedMode(usedMode: Boolean) = store.edit { it[Keys.USED_MODE] = usedMode }
    suspend fun setDismissedUpdateTag(tag: String?) = store.edit { prefs ->
        if (tag == null) prefs.remove(Keys.DISMISSED_UPDATE_TAG) else prefs[Keys.DISMISSED_UPDATE_TAG] = tag
    }
    suspend fun setLastUpdateCheckAt(epochMs: Long) = store.edit { it[Keys.LAST_UPDATE_CHECK_AT] = epochMs }

    /** null clears the override so newly-added providers show automatically. */
    suspend fun setVisibleProviderIds(ids: Set<String>?) = store.edit { prefs ->
        if (ids == null) {
            prefs[Keys.HAS_VISIBLE_OVERRIDE] = false
            prefs.remove(Keys.VISIBLE_PROVIDERS)
        } else {
            prefs[Keys.HAS_VISIBLE_OVERRIDE] = true
            prefs[Keys.VISIBLE_PROVIDERS] = ids
        }
    }
}

/** This is a LOCAL display preference only — it never changes which
 * providers are active on the PC. */
fun visibleProviders(all: List<ProviderUsage>, allowList: Set<String>?): List<ProviderUsage> {
    val enabled = all.filter { it.enabled }
    return if (allowList == null) enabled else enabled.filter { it.id in allowList }
}
