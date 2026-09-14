package com.shadelight.iausage.data

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class ProviderVisualsTest {
    @Test fun `known providers resolve a real icon`() {
        assertNotNull(providerVisual("anthropic", "Claude Code").icon)
        assertNotNull(providerVisual("openai", "Codex").icon)
        assertNotNull(providerVisual("antigravity", "Antigravity").icon)
    }

    @Test fun `an unknown provider never breaks - generic fallback plus derived code`() {
        val visual = providerVisual("some-new-vendor", "Some New Vendor")
        assertNull(visual.icon)
        assertEquals("SOM", visual.shortCode)
    }

    @Test fun `a name with no alphanumerics still yields a usable fallback code`() {
        assertEquals("AI", providerVisual("mystery", "!!!").shortCode)
    }

    @Test fun `known providers use the desktop tab codes`() {
        assertEquals("CLD", providerVisual("anthropic", "Claude Code").shortCode)
        assertEquals("CDX", providerVisual("openai", "Codex / ChatGPT").shortCode)
    }

    @Test fun `short names stay compact for small widgets`() {
        assertEquals("Claude", providerShortName("anthropic", "Claude Code"))
        assertEquals("Codex", providerShortName("openai", "Codex / ChatGPT"))
        assertEquals("Grok", providerShortName("grok", "Grok (xAI)"))
        assertEquals("Cursor", providerShortName("cursor", "Cursor"))
    }
}
