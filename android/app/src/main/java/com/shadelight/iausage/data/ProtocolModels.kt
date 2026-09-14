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

data class DashboardSnapshot(val generatedAt: String, val providers: List<ProviderUsage>) {
    companion object {
        fun fromJson(json: JSONObject): DashboardSnapshot {
            require(json.getInt("schemaVersion") == SUPPORTED_SCHEMA_VERSION) { "El dashboard usa un esquema no compatible." }
            val source = json.optJSONArray("providers") ?: JSONArray()
            return DashboardSnapshot(json.getString("generatedAt"), (0 until source.length()).map { ProviderUsage.fromJson(source.getJSONObject(it)) })
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
            )
        }
    }
}

data class UsageQuota(val id: String, val label: String, val usedPercent: Double?, val resetAt: String?, val resetInSeconds: Long?, val stale: Boolean) {
    companion object {
        fun fromJson(json: JSONObject) = UsageQuota(
            id = json.getString("id"), label = json.getString("label"),
            usedPercent = json.optDouble("usedPercent").takeUnless { it.isNaN() },
            resetAt = json.optString("resetAt").takeIf { it.isNotBlank() },
            resetInSeconds = if (json.has("resetInSeconds") && !json.isNull("resetInSeconds")) json.getLong("resetInSeconds") else null,
            stale = json.optBoolean("stale", false),
        )
    }
}
