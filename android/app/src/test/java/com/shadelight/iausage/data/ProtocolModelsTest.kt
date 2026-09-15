package com.shadelight.iausage.data

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ProtocolModelsTest {
    @Test fun `old quota payload remains readable without pace fields`() {
        val quota = UsageQuota.fromJson(
            JSONObject(
                """{"id":"session","label":"Sesión","usedPercent":25,"resetAt":"2026-09-14T15:00:00Z","resetInSeconds":3600,"stale":false}""",
            ),
        )
        assertEquals("session", quota.id)
        assertNull(quota.windowType)
        assertNull(quota.pace)
        assertNull(quota.groupId)
        assertEquals("always", quota.visible)
    }

    @Test fun `enriched quota payload exposes window and pace`() {
        val quota = UsageQuota.fromJson(
            JSONObject(
                """{
                    "id":"session",
                    "label":"Sesión",
                    "windowType":"session",
                    "usedPercent":60,
                    "resetAt":"2026-09-14T15:00:00Z",
                    "resetInSeconds":18000,
                    "stale":false,
                    "pace":{
                        "expectedUsedPercent":40,
                        "actualUsedPercent":60,
                        "deltaPercent":20,
                        "willLastToReset":false,
                        "estimatedExhaustedAt":"2026-09-14T13:00:00Z"
                    }
                }""".trimIndent(),
            ),
        )
        assertEquals("session", quota.windowType)
        assertEquals(20.0, quota.pace?.deltaPercent ?: 0.0, 0.001)
        assertFalse(quota.pace?.willLastToReset ?: true)
        assertEquals("2026-09-14T13:00:00Z", quota.pace?.estimatedExhaustedAt)
    }

    @Test fun `quota group metadata is additive and optional`() {
        val quota = UsageQuota.fromJson(
            JSONObject(
                """{
                    "id":"gemini_models_weekly",
                    "label":"weekly",
                    "windowType":"weekly",
                    "usedPercent":5.48,
                    "resetAt":null,
                    "resetInSeconds":null,
                    "stale":false,
                    "groupId":"gemini_models",
                    "groupLabel":"Gemini Models",
                    "models":["Gemini Flash","Gemini Pro"],
                    "visible":"always"
                }""",
            ),
        )
        assertEquals("gemini_models", quota.groupId)
        assertEquals("Gemini Models", quota.groupLabel)
        assertEquals(listOf("Gemini Flash", "Gemini Pro"), quota.models)
        assertEquals("always", quota.visible)
    }

    @Test fun `nullable pace decision is preserved`() {
        val pace = UsagePace.fromJson(
            JSONObject(
                """{"expectedUsedPercent":40,"actualUsedPercent":40,"deltaPercent":0,"willLastToReset":null,"estimatedExhaustedAt":null}""",
            ),
        )
        assertNull(pace.willLastToReset)
        assertNull(pace.estimatedExhaustedAt)
        assertTrue(pace.deltaPercent == 0.0)
    }

    @Test fun `dashboard parses shared limiting recommendation`() {
        val dashboard = DashboardSnapshot.fromJson(
            JSONObject(
                """{
                  "schemaVersion":1,
                  "generatedAt":"2026-09-14T12:00:00Z",
                  "providers":[],
                  "recommendation":{
                    "severity":"critical",
                    "action":"switch",
                    "fromId":"anthropic",
                    "toId":"openai",
                    "toName":"Codex",
                    "reason":"quota_near_exhaustion",
                    "confidence":0.9,
                    "limitingQuota":{
                      "id":"weekly","label":"Semanal","windowType":"weekly",
                      "usedPercent":99,"availablePercent":1,"resetAt":null,
                      "resetInSeconds":126000,"expectedUsedPercent":79,
                      "deltaPercent":20,"estimatedExhaustedAt":null,
                      "exhaustsBeforeResetSeconds":122400
                    },
                    "candidates":[]
                  }
                }""".trimIndent(),
            ),
        )
        assertEquals("critical", dashboard.recommendation?.severity)
        assertEquals("weekly", dashboard.recommendation?.limitingQuota?.id)
        assertEquals(1.0, dashboard.recommendation?.limitingQuota?.availablePercent ?: 0.0, 0.001)
    }
}
