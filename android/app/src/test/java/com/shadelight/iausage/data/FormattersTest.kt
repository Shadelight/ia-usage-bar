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

    @Test fun `duration includes leftover minutes when there are days`() {
        assertEquals("3 d 16 h 5 min", Formatters.formatDuration(3 * 86_400 + 16 * 3600 + 5 * 60))
    }

    @Test fun `short duration fits the 2x1 widget`() {
        assertEquals("39m", Formatters.formatDurationShort(39 * 60))
        assertEquals("3h 50m", Formatters.formatDurationShort(3 * 3600 + 50 * 60))
        assertEquals("2h", Formatters.formatDurationShort(2 * 3600))
        assertEquals("4d 19h", Formatters.formatDurationShort(4 * 86_400 + 19 * 3600))
        assertEquals("1m", Formatters.formatDurationShort(10))
    }

    @Test fun `available is the complement of used, clamped`() {
        assertEquals(79.0, Formatters.availablePercent(21.0)!!, 0.001)
        assertEquals(0.0, Formatters.availablePercent(140.0)!!, 0.001) // clamp above 100% used
        assertEquals(100.0, Formatters.availablePercent(-10.0)!!, 0.001) // clamp below 0% used
        assertNull(Formatters.availablePercent(null))
    }

    @Test fun `summaryPercents matches the cross-platform rounding contract`() {
        for ((exact, used, remaining) in Formatters.ROUNDING_VECTORS) {
            assertEquals(used to remaining, Formatters.summaryPercents(exact))
        }
    }

    @Test fun `percentText switches between used and remaining`() {
        assertEquals("21%", Formatters.percentText(21.0, usedMode = true))
        assertEquals("79%", Formatters.percentText(21.0, usedMode = false))
        assertEquals("6%", Formatters.percentText(5.5, usedMode = true))
        assertEquals("94%", Formatters.percentText(5.5, usedMode = false))
        assertEquals("—", Formatters.percentText(null, usedMode = true))
    }

    @Test fun `groupsFromQuotas keeps Antigravity 2x2 and skips details`() {
        fun q(
            id: String,
            label: String,
            used: Double,
            groupId: String,
            groupLabel: String,
            models: List<String> = emptyList(),
            visible: String = "always",
            windowType: String? = "weekly",
        ) = UsageQuota(
            id = id, label = label, usedPercent = used, resetAt = null, resetInSeconds = null, stale = false,
            windowType = windowType, groupId = groupId, groupLabel = groupLabel, models = models, visible = visible,
        )
        val groups = Formatters.groupsFromQuotas(
            listOf(
                q("gemini_models_weekly", "weekly", 5.48, "gemini_models", "Gemini Models", listOf("Gemini Flash")),
                q("gemini_models_5h", "5h", 14.88, "gemini_models", "Gemini Models", windowType = "5h"),
                q("claude_gpt_models_weekly", "weekly", 0.0, "claude_gpt_models", "Claude + GPT"),
                q("claude_gpt_models_5h", "5h", 0.0, "claude_gpt_models", "Claude + GPT", windowType = "5h"),
                q("total", "Total", 40.0, "cursor", "Cursor", visible = "details"),
            ),
        )
        assertEquals(2, groups.size)
        assertEquals("gemini_models", groups[0].id)
        assertEquals(2, groups[0].quotas.size)
        assertEquals(listOf("Gemini Flash"), groups[0].models)
        assertEquals("Claude + GPT", groups[1].label)
        assertEquals(15 to 85, Formatters.groupSummary(groups[0]))
    }

    @Test fun `live reset countdown is derived from absolute reset time`() {
        val now = java.time.Instant.parse("2026-09-14T10:00:00Z").toEpochMilli()
        assertEquals(3_600L, Formatters.secondsUntil("2026-09-14T11:00:00Z", now))
        assertEquals(0L, Formatters.secondsUntil("2026-09-14T09:00:00Z", now))
        assertNull(Formatters.secondsUntil("not-a-date", now))
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
