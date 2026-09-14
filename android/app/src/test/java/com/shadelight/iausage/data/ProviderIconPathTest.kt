package com.shadelight.iausage.data

import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/** Widgets are drawn by the launcher with the framework's native PathParser,
 * which rejects compact SVG arc flags such as "a.8.8 0 00-.8 0" — the icon
 * silently renders blank on the home screen, while Compose (the app itself)
 * parses the same file fine. Keep every provider icon in the fully
 * separated form: single-space tokens, one command letter or number each. */
class ProviderIconPathTest {
    // Checked token by token: one regex over a whole long path overflows the
    // JVM regex stack.
    private val token = Regex("""[A-Za-z]|[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?""")
    private val separated = { path: String -> path.split(' ').all { token.matches(it) } }

    @Test fun `provider icon paths are parseable by the framework PathParser`() {
        val drawables = File("src/main/res/drawable").listFiles { file -> file.name.startsWith("ic_provider_") }.orEmpty()
        assertTrue("no provider icons found from ${File(".").absolutePath}", drawables.isNotEmpty())
        val offenders = drawables.flatMap { file ->
            Regex("""android:pathData="([^"]*)"""").findAll(file.readText())
                .map { it.groupValues[1] }
                .filterNot { separated(it) }
                .map { "${file.name}: ${it.take(40)}…" }
                .toList()
        }
        assertTrue("paths not in separated form:\n" + offenders.joinToString("\n"), offenders.isEmpty())
    }
}
