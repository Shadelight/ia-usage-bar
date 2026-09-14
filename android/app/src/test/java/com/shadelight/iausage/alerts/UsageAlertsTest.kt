package com.shadelight.iausage.alerts

import com.shadelight.iausage.data.ProviderUsage
import com.shadelight.iausage.data.UsageQuota
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

private fun provider(
    id: String,
    used: Double?,
    resetAt: String? = "2026-09-14T15:00:00Z",
    status: String? = "connected",
) = ProviderUsage(
    id = id,
    name = "Name $id",
    enabled = true,
    connectionStatus = status,
    connectionReason = null,
    stale = false,
    updatedAt = null,
    quotas = listOf(UsageQuota(id = "session", label = "Sesión", usedPercent = used, resetAt = resetAt, resetInSeconds = null, stale = false)),
)

private fun refresh(state: AlertState, vararg providers: ProviderUsage) = UsageAlerts.onRefreshed(state, providers.toList())

class UsageAlertsTest {
    @Test fun `first sighting is recorded silently`() {
        val (alerts, state) = refresh(AlertState(), provider("claude", 95.0))
        assertTrue(alerts.isEmpty())
        assertEquals(95.0, state.providers["claude"]?.usedPercent)
    }

    @Test fun `a reset needs the reset time to jump and usage to drop`() {
        val (_, seen) = refresh(AlertState(), provider("codex", 80.0, "2026-09-14T15:00:00Z"))
        val (alerts, _) = refresh(seen, provider("codex", 0.0, "2026-09-14T20:00:00+00:00"))
        val reset = alerts.single()
        assertTrue(reset is UsageAlert.Reset)
        assertEquals("Name codex se reinició", reset.title)
        assertEquals("100% disponible.", reset.body)
    }

    @Test fun `a sliding window creeping forward is not a reset`() {
        val (_, seen) = refresh(AlertState(), provider("opencode", 40.0, "2026-09-14T15:00:00Z"))
        val (alerts, _) = refresh(seen, provider("opencode", 42.0, "2026-09-14T15:10:00Z"))
        assertTrue(alerts.isEmpty())
    }

    @Test fun `near limit and exhausted fire once per window`() {
        val (_, s0) = refresh(AlertState(), provider("claude", 80.0))
        val (near, s1) = refresh(s0, provider("claude", 91.0))
        assertTrue(near.single() is UsageAlert.NearLimit)
        val (quiet, s2) = refresh(s1, provider("claude", 93.0))
        assertTrue(quiet.isEmpty())
        val (gone, s3) = refresh(s2, provider("claude", 100.0))
        assertTrue(gone.single() is UsageAlert.Exhausted)
        val (again, _) = refresh(s3, provider("claude", 100.0))
        assertTrue(again.isEmpty())
    }

    @Test fun `a reset re-arms the limit alerts`() {
        val (_, s0) = refresh(AlertState(), provider("claude", 85.0, "2026-09-14T15:00:00Z"))
        val (_, s1) = refresh(s0, provider("claude", 95.0, "2026-09-14T15:00:00Z"))
        val (_, s2) = refresh(s1, provider("claude", 5.0, "2026-09-14T20:00:00Z"))
        val (alerts, _) = refresh(s2, provider("claude", 92.0, "2026-09-14T20:00:00Z"))
        assertTrue(alerts.single() is UsageAlert.NearLimit)
    }

    @Test fun `best option change needs a real margin`() {
        val (_, s0) = refresh(AlertState(), provider("claude", 40.0), provider("codex", 50.0))
        assertEquals("claude", s0.bestId)
        val (small, s1) = refresh(s0, provider("claude", 50.0), provider("codex", 45.0))
        assertTrue(small.isEmpty())
        assertEquals("claude", s1.bestId)
        val (big, s2) = refresh(s1, provider("claude", 80.0), provider("codex", 45.0))
        val change = big.single()
        assertTrue(change is UsageAlert.BestChanged)
        assertEquals("Ahora Name codex es la mejor opción", change.title)
        assertEquals("codex", s2.bestId)
    }

    @Test fun `losing the PC alerts once after repeated failures and a success re-arms it`() {
        var state = AlertState()
        val fired = (1..5).flatMap {
            val (alerts, next) = UsageAlerts.onRefreshFailed(state)
            state = next
            alerts
        }
        assertEquals(1, fired.size)
        assertTrue(fired.single() === UsageAlert.SyncLost)
        val (_, recovered) = refresh(state, provider("claude", 10.0))
        assertEquals(0, recovered.syncFailures)
        assertFalse(recovered.syncAlerted)
    }

    @Test fun `disconnected providers never alert`() {
        val (_, s0) = refresh(AlertState(), provider("claude", 80.0))
        val (alerts, _) = refresh(s0, provider("claude", 95.0, status = "needs_auth"))
        assertTrue(alerts.isEmpty())
    }

    @Test fun `state survives a persistence round trip and bad input starts over`() {
        val (_, s0) = refresh(AlertState(), provider("claude", 85.0), provider("codex", null))
        val (_, s1) = refresh(s0, provider("claude", 91.0))
        val saved = s1.copy(syncFailures = 2, syncAlerted = true)
        assertEquals(saved, AlertStateCodec.decode(AlertStateCodec.encode(saved)))
        assertEquals(AlertState(), AlertStateCodec.decode("not json"))
        assertEquals(AlertState(), AlertStateCodec.decode(null))
    }
}
