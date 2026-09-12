package com.shadelight.iausage.data

import android.content.Context
import com.shadelight.iausage.BuildConfig
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

class UsageSyncRepository(private val context: Context) {
    private val store = SecureStateStore(context.applicationContext)
    private val crypto = ProtocolCrypto()

    fun cachedPayload(): SyncPayload? = store.payload()?.let { runCatching { SyncPayload.fromJson(JSONObject(it)) }.getOrNull() }

    fun pair(pairing: PairingInfo, passphrase: CharArray): SyncPayload {
        val verified = fetch(pairing, passphrase)
        store.savePairing(pairing, passphrase)
        store.savePayload(verified.second)
        return verified.first
    }

    fun refresh(): SyncPayload {
        val pairing = store.pairing() ?: error("No hay un PC vinculado.")
        val passphrase = store.passphrase() ?: error("La passphrase segura ya no está disponible.")
        val verified = fetch(pairing, passphrase)
        store.savePayload(verified.second)
        return verified.first
    }

    fun disconnect() = store.clear()
    fun pairing() = store.pairing()

    private fun fetch(pairing: PairingInfo, passphrase: CharArray): Pair<SyncPayload, String> {
        val baseUrl = "http://${bracketedHost(pairing.host)}:${pairing.port}"
        val meta = ServerMeta.fromJson(getJson("$baseUrl/v1/meta"))
        require(meta.schemaVersion == SUPPORTED_SCHEMA_VERSION && meta.blobVersion == SUPPORTED_BLOB_VERSION) { "El PC usa una versión de sync no compatible." }
        require(meta.lan) { "El servidor del PC no está expuesto a la red local." }
        require(meta.deviceId.equals(pairing.deviceId, true) && meta.fingerprint == pairing.fingerprint) { "El PC no coincide con el QR. No continúes el pareo." }
        require(versionAtLeast(BuildConfig.VERSION_NAME, pairing.minAppVersion)) { "Actualiza IA Usage en Android para este PC." }
        val blob = EncryptedBlob.fromJson(getJson("$baseUrl/v1/snapshot"))
        val raw = crypto.decrypt(blob, passphrase)
        val payload = SyncPayload.fromJson(JSONObject(raw))
        require(payload.deviceId.equals(pairing.deviceId, true)) { "El snapshot no pertenece al PC vinculado." }
        return payload to raw
    }

    private fun getJson(url: String): JSONObject {
        val connection = (URL(url).openConnection() as HttpURLConnection).apply {
            connectTimeout = 10_000
            readTimeout = 20_000
            requestMethod = "GET"
            setRequestProperty("Accept", "application/json")
        }
        return try {
            val code = connection.responseCode
            val body = (if (code in 200..299) connection.inputStream else connection.errorStream)?.bufferedReader()?.use { it.readText() }.orEmpty()
            require(code in 200..299) { "El PC respondió HTTP $code." }
            JSONObject(body)
        } finally {
            connection.disconnect()
        }
    }

    private fun bracketedHost(host: String) = if (host.contains(':') && !host.startsWith('[')) "[$host]" else host
    private fun versionAtLeast(actual: String, minimum: String): Boolean {
        fun parse(value: String) = value.substringBefore('-').split('.').map { it.toIntOrNull() ?: 0 }.let { it + List((3 - it.size).coerceAtLeast(0)) { 0 } }
        return parse(actual).zip(parse(minimum)).firstOrNull { it.first != it.second }?.let { it.first > it.second } ?: true
    }
}
