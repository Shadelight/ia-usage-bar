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
}
