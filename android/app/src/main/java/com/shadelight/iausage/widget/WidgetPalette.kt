package com.shadelight.iausage.widget

import androidx.compose.ui.graphics.Color

/** The widget always draws its own dark card (launcher wallpapers vary too
 * much to trust a light surface). Shared by the Glance widget and the
 * configuration preview so both look identical. Coral is the IA Usage
 * accent; green/amber/red are used only for status. */
object WidgetPalette {
    val Background = Color(0xFF18181B)
    val Surface = Color(0xFF26262B)
    val Track = Color(0xFF34343A)
    val TextPrimary = Color(0xFFF5F5F5)
    val TextMuted = Color(0xFFA1A1AA)
    val Accent = Color(0xFFF9734F)
    val Ok = Color(0xFF4ADE80)
    val Warn = Color(0xFFFBBF24)
    val Critical = Color(0xFFF87171)

    fun status(level: UsageLevel): Color = when (level) {
        UsageLevel.OK -> Ok
        UsageLevel.WARN -> Warn
        UsageLevel.CRITICAL -> Critical
        UsageLevel.UNKNOWN -> TextMuted
    }

    /** Bars stay coral and only turn red once the quota is critical. */
    fun bar(level: UsageLevel): Color = if (level == UsageLevel.CRITICAL) Critical else Accent
}
