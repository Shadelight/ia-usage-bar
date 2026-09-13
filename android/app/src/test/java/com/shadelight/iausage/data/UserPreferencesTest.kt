package com.shadelight.iausage.data

import org.junit.Assert.assertEquals
import org.junit.Test

private fun provider(id: String, enabled: Boolean = true) =
    ProviderUsage(id = id, name = id, enabled = enabled, connectionStatus = "connected", connectionReason = null, stale = false, updatedAt = null, quotas = emptyList())

class UserPreferencesTest {
    @Test fun `null allow-list shows every enabled provider`() {
        val all = listOf(provider("a"), provider("b", enabled = false), provider("c"))
        assertEquals(listOf("a", "c"), visibleProviders(all, null).map { it.id })
    }

    @Test fun `an explicit allow-list filters down to it`() {
        val all = listOf(provider("a"), provider("b"), provider("c"))
        assertEquals(listOf("a", "c"), visibleProviders(all, setOf("a", "c")).map { it.id })
    }

    @Test fun `a newly added provider not in the allow-list stays hidden until picked`() {
        val all = listOf(provider("a"), provider("brand-new"))
        assertEquals(listOf("a"), visibleProviders(all, setOf("a")).map { it.id })
    }
}
