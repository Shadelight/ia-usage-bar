package com.shadelight.iausage.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

// Same family as the desktop app's dark palette, so the two feel related.
// ponytail: no light/system toggle yet, dark is the only look for now.
private val Background = Color(0xFF18181B)
private val Surface = Color(0xFF202023)
private val SurfaceElevated = Color(0xFF28282D)
private val Border = Color(0xFF34343A)
private val TextPrimary = Color(0xFFF5F5F5)
private val TextSecondary = Color(0xFFA1A1AA)
private val Accent = Color(0xFFF9734F)
private val ErrorColor = Color(0xFFF87171)

private val IaUsageDarkScheme = darkColorScheme(
    background = Background,
    surface = Surface,
    surfaceVariant = SurfaceElevated,
    onBackground = TextPrimary,
    onSurface = TextPrimary,
    onSurfaceVariant = TextSecondary,
    primary = Accent,
    onPrimary = Color(0xFF1A1A1A),
    outline = Border,
    error = ErrorColor,
)

@Composable
fun IaUsageTheme(darkTheme: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit) {
    val scheme = if (darkTheme) IaUsageDarkScheme else lightColorScheme(primary = Accent, error = ErrorColor)
    MaterialTheme(colorScheme = scheme, content = content)
}
