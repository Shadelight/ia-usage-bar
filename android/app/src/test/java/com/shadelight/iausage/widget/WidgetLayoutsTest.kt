package com.shadelight.iausage.widget

import com.shadelight.iausage.data.ProviderUsage
import com.shadelight.iausage.data.UsageQuota
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

private fun quota(used: Double?, label: String = "Sesión", resetIn: Long? = null) =
    UsageQuota(id = label, label = label, usedPercent = used, resetAt = null, resetInSeconds = resetIn, stale = false)

private fun provider(id: String, used: Double?, enabled: Boolean = true) = ProviderUsage(
    id = id,
    name = id,
    enabled = enabled,
    connectionStatus = "connected",
    connectionReason = null,
    stale = false,
    updatedAt = null,
    quotas = listOf(quota(used)),
)

class WidgetLayoutsTest {
    @Test fun `each launcher footprint gets its own layout`() {
        assertEquals(WidgetLayout.TINY, widgetLayout(57f, 57f, WIDGET_MODE_AUTO))
        assertEquals(WidgetLayout.WIDE, widgetLayout(130f, 57f, WIDGET_MODE_AUTO))
        assertEquals(WidgetLayout.SQUARE, widgetLayout(130f, 130f, WIDGET_MODE_AUTO))
        assertEquals(WidgetLayout.LARGE_SINGLE, widgetLayout(250f, 130f, "anthropic"))
        assertEquals(WidgetLayout.LARGE_COMPARE, widgetLayout(250f, 130f, WIDGET_MODE_COMPARE))
        // Narrow but tall: still one metric, there is no room for a second column.
        assertEquals(WidgetLayout.TINY, widgetLayout(57f, 250f, WIDGET_MODE_COMPARE))
    }

    @Test fun `usage level bands match the desktop alert thresholds`() {
        assertEquals(UsageLevel.OK, usageLevel(74.9))
        assertEquals(UsageLevel.WARN, usageLevel(75.0))
        assertEquals(UsageLevel.WARN, usageLevel(89.9))
        assertEquals(UsageLevel.CRITICAL, usageLevel(90.0))
        assertEquals(UsageLevel.UNKNOWN, usageLevel(null))
    }

    @Test fun `a fixed provider wins and auto picks the most room left`() {
        val list = listOf(provider("claude", 90.0), provider("codex", 10.0))
        assertEquals("claude", pickProvider(list, "claude")?.first?.id)
        assertEquals("codex", pickProvider(list, WIDGET_MODE_AUTO)?.first?.id)
        assertEquals("codex", pickProvider(list, "removed-provider")?.first?.id)
        assertNull(pickProvider(emptyList(), WIDGET_MODE_AUTO))
    }

    @Test fun `compare shows at most three rows and counts the rest`() {
        val list = (1..5).map { provider("p$it", 10.0) } + provider("off", 0.0, enabled = false)
        val all = compareRows(list, emptySet())
        assertEquals(listOf("p1", "p2", "p3"), all.rows.map { it.id })
        assertEquals(2, all.hiddenCount)
        val picked = compareRows(list, setOf("p4", "p2"))
        assertEquals(listOf("p2", "p4"), picked.rows.map { it.id })
        assertEquals(0, picked.hiddenCount)
    }

    @Test fun `quota caption carries the reset only when enabled`() {
        val weekly = quota(40.0, "Semanal", resetIn = 2 * 3600)
        assertEquals("Semanal · 2 h", quotaCaption(weekly, WidgetConfig(showReset = true)))
        assertEquals("Semanal", quotaCaption(weekly, WidgetConfig(showReset = false)))
    }

    @Test fun `compact footer keeps one fact so it never truncates`() {
        val session = quota(40.0, resetIn = 3 * 3600)
        val config = WidgetConfig(showReset = true, showStatus = true)
        assertEquals("Reinicia en 3 h", widgetFooter(session, config, generatedAt = null, stale = false, compact = true))
        assertEquals("", widgetFooter(session, WidgetConfig(showReset = false, showStatus = false), null, stale = false, compact = true))
        // Stale data outranks the reset time.
        val stale = widgetFooter(session, config, generatedAt = "2026-09-14T10:00:00Z", stale = true, compact = true)
        assertEquals(true, stale.startsWith("⚠ datos de"))
    }

    @Test fun `saving a single-provider widget requires choosing the provider`() {
        assertEquals("Selecciona una IA", widgetDraftError(singleProvider = true, providerId = null))
        assertNull(widgetDraftError(singleProvider = true, providerId = "claude"))
        assertNull(widgetDraftError(singleProvider = false, providerId = null))
    }
}
