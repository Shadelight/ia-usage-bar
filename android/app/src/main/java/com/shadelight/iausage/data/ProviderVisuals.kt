package com.shadelight.iausage.data

import com.shadelight.iausage.R

/** Same brand icon set as Desktop/VSCode, converted from the canonical SVGs
 * under src/assets/providers into vector drawables (see
 * res/drawable/ic_provider_*.xml). A provider without a shipped icon never
 * breaks the UI: it falls back to a generic mark + a derived 3-letter code. */
data class ProviderVisual(val icon: Int?, val shortCode: String)

private val KNOWN_ICONS: Map<String, Int> = mapOf(
    "anthropic" to R.drawable.ic_provider_anthropic,
    "openai" to R.drawable.ic_provider_openai,
    "cursor" to R.drawable.ic_provider_cursor,
    "antigravity" to R.drawable.ic_provider_antigravity,
    "opencode_go" to R.drawable.ic_provider_opencode_go,
    "openrouter" to R.drawable.ic_provider_openrouter,
    "deepseek" to R.drawable.ic_provider_deepseek,
    "groq" to R.drawable.ic_provider_groq,
    "kimi" to R.drawable.ic_provider_kimi,
    "kilo" to R.drawable.ic_provider_kilo,
    "minimax" to R.drawable.ic_provider_minimax,
    "grok" to R.drawable.ic_provider_grok,
    "copilot" to R.drawable.ic_provider_copilot,
    "windsurf" to R.drawable.ic_provider_windsurf,
    "zai" to R.drawable.ic_provider_zai,
    "moonshot" to R.drawable.ic_provider_moonshot,
    "novita" to R.drawable.ic_provider_novita,
    "kiro" to R.drawable.ic_provider_kiro,
    "nous" to R.drawable.ic_provider_nous,
)

private fun fallbackShortCode(name: String): String {
    val letters = name.filter { it.isLetterOrDigit() }
    return (if (letters.length >= 3) letters.substring(0, 3) else letters.ifEmpty { "AI" }).uppercase()
}

fun providerVisual(id: String, name: String): ProviderVisual =
    ProviderVisual(icon = KNOWN_ICONS[id], shortCode = fallbackShortCode(name))
