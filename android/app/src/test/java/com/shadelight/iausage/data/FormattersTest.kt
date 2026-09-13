package com.shadelight.iausage.data

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

private fun quota(id: String = "session", used: Double? = 21.0, resetInSeconds: Long? = null, stale: Boolean = false) =
    UsageQuota(id = id, label = id, usedPercent = used, resetAt = null, resetInSeconds = resetInSeconds, stale = stale)

private fun provider(
    id: String = "openai",
    enabled: Boolean = true,
    status: String? = "connected",
    stale: Boolean = false,
    quotas: List<UsageQuota> = listOf(quota()),
) = ProviderUsage(id = id, name = id, enabled = enabled, connectionStatus = status, connectionReason = null, stale = stale, updatedAt = null, quotas = quotas)

class FormattersTest {
    @Test fun `duration formats without raw seconds`() {
        assertEquals("39 min", Formatters.formatDuration(39 * 60))
        assertEquals("4 h 31 min", Formatters.formatDuration(4 * 3600 + 31 * 60))
        assertEquals("3 d", Formatters.formatDuration(3 * 86_400))
        assertEquals("1 min", Formatters.formatDuration(10)) // never rounds down to 0
    }

    @Test fun `available is the complement of used, clamped`() {
        assertEquals(79.0, Formatters.availablePercent(21.0)!!, 0.001)
        assertEquals(0.0, Formatters.availablePercent(140.0)!!, 0.001) // clamp above 100% used
        assertEquals(100.0, Formatters.availablePercent(-10.0)!!, 0.001) // clamp below 0% used
        assertNull(Formatters.availablePercent(null))
    }

    @Test fun `percentText switches between used and available`() {
        assertEquals("21%", Formatters.percentText(21.0, usedMode = true))
        assertEquals("79%", Formatters.percentText(21.0, usedMode = false))
        assertEquals("—", Formatters.percentText(null, usedMode = true))
    }

    @Test fun `primaryQuota honors the preferred id and falls back safely`() {
        val p = provider(quotas = listOf(quota("five_hour", 18.0), quota("weekly", 30.0)))
        assertEquals("weekly", Formatters.primaryQuota(p, "weekly")?.id)
        assertEquals("five_hour", Formatters.primaryQuota(p, null)?.id)
        assertEquals("five_hour", Formatters.primaryQuota(p, "does_not_exist")?.id)
    }

    @Test fun `a stale quota marks the whole provider stale even if the provider flag is false`() {
        val p = provider(stale = false, quotas = listOf(quota(stale = true)))
        assertTrue(Formatters.isStale(p))
    }

    @Test fun `snapshot is stale if any provider is stale`() {
        val fresh = provider(id = "a", stale = false)
        val stale = provider(id = "b", stale = true)
        assertTrue(Formatters.isSnapshotStale(listOf(fresh, stale)))
        assertTrue(!Formatters.isSnapshotStale(listOf(fresh)))
    }

    @Test fun `best available provider is the one with the most room left`() {
        val low = provider(id = "low", quotas = listOf(quota(used = 90.0)))
        val high = provider(id = "high", quotas = listOf(quota(used = 10.0)))
        val result = Formatters.bestAvailableProvider(listOf(low, high)) { null }
        assertEquals("high", result?.first?.id)
    }

    @Test fun `best available provider ignores disabled and disconnected providers`() {
        val disabled = provider(id = "disabled", enabled = false, quotas = listOf(quota(used = 0.0)))
        val disconnected = provider(id = "disconnected", status = "needs_auth", quotas = listOf(quota(used = 0.0)))
        val ok = provider(id = "ok", quotas = listOf(quota(used = 50.0)))
        val result = Formatters.bestAvailableProvider(listOf(disabled, disconnected, ok)) { null }
        assertEquals("ok", result?.first?.id)
    }
}
