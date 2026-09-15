package com.shadelight.iausage.data

import org.json.JSONArray
import org.json.JSONObject

const val SUPPORTED_SCHEMA_VERSION = 1
const val SUPPORTED_BLOB_VERSION = 1
const val SUPPORTED_ALGORITHM = "xchacha20poly1305+argon2id"
const val SUPPORTED_ALGORITHM_V2 = "xchacha20poly1305"

data class PairingInfo(
    val host: String,
    val port: Int,
    val deviceId: String,
    val fingerprint: String,
    val minAppVersion: String,
    val token: String? = null,
    val secret: ByteArray? = null,
)

data class ServerMeta(
    val schemaVersion: Int,
    val blobVersion: Int,
    val deviceId: String,
    val fingerprint: String,
    val appVersion: String,
    val lan: Boolean,
) {
    companion object {
        fun fromJson(json: JSONObject) = ServerMeta(
            schemaVersion = json.getInt("schemaVersion"),
            blobVersion = json.getInt("blobVersion"),
            deviceId = json.getString("deviceId"),
            fingerprint = json.getString("fingerprint"),
            appVersion = json.optString("appVersion", ""),
            lan = json.optBoolean("lan", false),
        )
    }
}

data class EncryptedBlob(val version: Int, val algorithm: String, val salt: String, val nonce: String, val ciphertext: String) {
    companion object {
        fun fromJson(json: JSONObject) = EncryptedBlob(
            version = json.getInt("v"), algorithm = json.getString("alg"), salt = json.getString("salt"),
            nonce = json.getString("nonce"), ciphertext = json.getString("ciphertext"),
        )
    }
}

data class SyncPayload(val schemaVersion: Int, val deviceId: String, val generatedAt: String, val snapshot: DashboardSnapshot) {
    companion object {
        fun fromJson(json: JSONObject): SyncPayload {
            val schema = json.getInt("schemaVersion")
            require(schema == SUPPORTED_SCHEMA_VERSION) { "El snapshot usa un esquema no compatible." }
            return SyncPayload(schema, json.getString("deviceId"), json.getString("generatedAt"), DashboardSnapshot.fromJson(json.getJSONObject("snapshot")))
        }
    }
}

data class DashboardSnapshot(
    val generatedAt: String,
    val providers: List<ProviderUsage>,
    val recommendation: Recommendation? = null,
) {
    companion object {
        fun fromJson(json: JSONObject): DashboardSnapshot {
            require(json.getInt("schemaVersion") == SUPPORTED_SCHEMA_VERSION) { "El dashboard usa un esquema no compatible." }
            val source = json.optJSONArray("providers") ?: JSONArray()
            return DashboardSnapshot(
                json.getString("generatedAt"),
                (0 until source.length()).map { ProviderUsage.fromJson(source.getJSONObject(it)) },
                json.optJSONObject("recommendation")?.let(Recommendation::fromJson),
            )
        }
    }
}

data class ProviderUsage(
    val id: String,
    val name: String,
    val enabled: Boolean,
    val connectionStatus: String?,
    val connectionReason: String?,
    val stale: Boolean,
    val updatedAt: String?,
    val quotas: List<UsageQuota>,
    val credits: CreditsSummary? = null,
    val availability: String? = null,
    val plan: String? = null,
) {
    companion object {
        fun fromJson(json: JSONObject): ProviderUsage {
            val connection = json.optJSONObject("connection")
            val quotas = json.optJSONArray("quotas") ?: JSONArray()
            return ProviderUsage(
                id = json.getString("id"), name = json.getString("name"), enabled = json.optBoolean("enabled", true),
                connectionStatus = connection?.optString("status"), connectionReason = connection?.optString("reason"),
                stale = json.optBoolean("stale", false), updatedAt = json.optString("updatedAt").takeIf { it.isNotBlank() },
                quotas = (0 until quotas.length()).map { UsageQuota.fromJson(quotas.getJSONObject(it)) },
                credits = json.optJSONObject("credits")?.let(CreditsSummary::fromJson),
                availability = json.optString("availability").takeIf { it.isNotBlank() },
                plan = json.optString("plan").takeIf { it.isNotBlank() },
            )
        }
    }
}

data class CreditsSummary(
    val remaining: Double,
    val resetsAvailable: Long?,
    val fetchedAt: String?,
    val stale: Boolean,
    val resetsStale: Boolean,
) {
    companion object {
        fun fromJson(json: JSONObject) = CreditsSummary(
            remaining = json.getDouble("remaining"),
            resetsAvailable = json.optLongOrNull("resetsAvailable"),
            fetchedAt = json.optString("fetchedAt").takeIf { it.isNotBlank() },
            stale = json.optBoolean("stale", false),
            resetsStale = json.optBoolean("resetsStale", false),
        )
    }
}

data class UsagePace(
    val expectedUsedPercent: Double,
    val actualUsedPercent: Double,
    val deltaPercent: Double,
    val willLastToReset: Boolean?,
    val estimatedExhaustedAt: String?,
) {
    companion object {
        fun fromJson(json: JSONObject) = UsagePace(
            expectedUsedPercent = json.optDouble("expectedUsedPercent", 0.0),
            actualUsedPercent = json.optDouble("actualUsedPercent", 0.0),
            deltaPercent = json.optDouble("deltaPercent", 0.0),
            willLastToReset = if (json.has("willLastToReset") && !json.isNull("willLastToReset")) {
                json.getBoolean("willLastToReset")
            } else {
                null
            },
            estimatedExhaustedAt = json.optString("estimatedExhaustedAt").takeIf { it.isNotBlank() },
        )
    }
}

data class UsageQuota(
    val id: String,
    val label: String,
    val usedPercent: Double?,
    val resetAt: String?,
    val resetInSeconds: Long?,
    val stale: Boolean,
    val windowType: String? = null,
    val pace: UsagePace? = null,
    val groupId: String? = null,
    val groupLabel: String? = null,
    val models: List<String> = emptyList(),
    val visible: String = "always",
) {
    companion object {
        fun fromJson(json: JSONObject) = UsageQuota(
            id = json.getString("id"), label = json.getString("label"),
            usedPercent = json.optDouble("usedPercent").takeUnless { it.isNaN() },
            resetAt = json.optString("resetAt").takeIf { it.isNotBlank() },
            resetInSeconds = if (json.has("resetInSeconds") && !json.isNull("resetInSeconds")) json.getLong("resetInSeconds") else null,
            stale = json.optBoolean("stale", false),
            windowType = json.optString("windowType").takeIf { it.isNotBlank() },
            pace = json.optJSONObject("pace")?.let(UsagePace::fromJson),
            groupId = json.optString("groupId").takeIf { it.isNotBlank() },
            groupLabel = json.optString("groupLabel").takeIf { it.isNotBlank() },
            models = json.optJSONArray("models")?.let { arr -> (0 until arr.length()).map { arr.optString(it) } } ?: emptyList(),
            visible = json.optString("visible").takeIf { it.isNotBlank() } ?: "always",
        )
    }
}

data class LimitingQuota(
    val id: String,
    val label: String,
    val windowType: String,
    val usedPercent: Double,
    val availablePercent: Double,
    val resetAt: String?,
    val resetInSeconds: Long?,
    val expectedUsedPercent: Double?,
    val deltaPercent: Double?,
    val estimatedExhaustedAt: String?,
    val exhaustsBeforeResetSeconds: Long?,
) {
    companion object {
        fun fromJson(json: JSONObject) = LimitingQuota(
            id = json.getString("id"),
            label = json.getString("label"),
            windowType = json.optString("windowType", "custom"),
            usedPercent = json.optDouble("usedPercent", 0.0),
            availablePercent = json.optDouble("availablePercent", 0.0),
            resetAt = json.optString("resetAt").takeIf { it.isNotBlank() },
            resetInSeconds = json.optLongOrNull("resetInSeconds"),
            expectedUsedPercent = json.optDoubleOrNull("expectedUsedPercent"),
            deltaPercent = json.optDoubleOrNull("deltaPercent"),
            estimatedExhaustedAt = json.optString("estimatedExhaustedAt").takeIf { it.isNotBlank() },
            exhaustsBeforeResetSeconds = json.optLongOrNull("exhaustsBeforeResetSeconds"),
        )
    }
}

data class Recommendation(
    val severity: String,
    val action: String,
    val fromId: String,
    val toId: String?,
    val toName: String?,
    val reason: String,
    val limitingQuota: LimitingQuota?,
    val confidence: Double,
) {
    companion object {
        fun fromJson(json: JSONObject) = Recommendation(
            severity = json.optString("severity", "healthy"),
            action = json.optString("action", "insufficient_data"),
            fromId = json.optString("fromId", ""),
            toId = json.optString("toId").takeIf { it.isNotBlank() },
            toName = json.optString("toName").takeIf { it.isNotBlank() },
            reason = json.optString("reason", "insufficient_data"),
            limitingQuota = json.optJSONObject("limitingQuota")?.let(LimitingQuota::fromJson),
            confidence = json.optDouble("confidence", 0.0),
        )
    }
}

private fun JSONObject.optLongOrNull(name: String): Long? =
    if (has(name) && !isNull(name)) getLong(name) else null

private fun JSONObject.optDoubleOrNull(name: String): Double? =
    if (has(name) && !isNull(name)) getDouble(name) else null
