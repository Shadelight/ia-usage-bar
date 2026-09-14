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
        val secret = store.deviceSecret()
        val (payload, raw) = if (secret != null) {
            val clientDeviceId = store.clientDeviceId() ?: error("Falta el identificador de este dispositivo.")
            fetchV2(pairing, clientDeviceId, secret)
        } else {
            val passphrase = store.passphrase() ?: error("La frase secreta segura ya no está disponible.")
            fetch(pairing, passphrase)
        }
        store.savePayload(raw)
        return payload
    }

    fun pairV2(pairing: PairingInfo): SyncPayload {
        val secret = requireNotNull(pairing.secret) { "El QR no contiene un secreto de pareo." }
        val token = requireNotNull(pairing.token) { "El QR no contiene un token de pareo." }
        val clientDeviceId = store.ensureClientDeviceId()
        val name = sanitizedDeviceName()
        postPair(pairing, token, clientDeviceId, name)
        // Persist the credential the instant the desktop confirms pairing —
        // BEFORE fetching the first snapshot. If postPair succeeds but
        // fetchV2 then throws (transient network blip), the desktop has
        // already committed the device row and secret; if we saved nothing
        // here the phone would have no way back in (the consumed token
        // can't be reused) while the desktop's device row lingers forever.
        // With the credential saved, refresh()'s existing V2 path retries
        // the snapshot fetch later using the now-stored secret.
        // A copy: savePairingV2 zeroes the array it gets, and `secret` is
        // still needed to decrypt the first snapshot right below.
        store.savePairingV2(pairing, clientDeviceId, secret.copyOf())
        val (payload, raw) = fetchV2(pairing, clientDeviceId, secret)
        store.savePayload(raw)
        return payload
    }

    fun disconnect() = store.clear()
    fun clearCachedPayload() = store.clearPayload()
    fun pairing() = store.pairing()

    private fun postPair(pairing: PairingInfo, token: String, clientDeviceId: String, name: String) {
        val baseUrl = "http://${bracketedHost(pairing.host)}:${pairing.port}"
        val body = JSONObject().apply {
            put("token", token)
            put("clientDeviceId", clientDeviceId)
            put("name", name)
        }
        val connection = (URL("$baseUrl/v2/pair").openConnection() as HttpURLConnection).apply {
            connectTimeout = 10_000
            readTimeout = 20_000
            requestMethod = "POST"
            doOutput = true
            setRequestProperty("Content-Type", "application/json")
        }
        try {
            connection.outputStream.use { it.write(body.toString().toByteArray(Charsets.UTF_8)) }
            when (val code = connection.responseCode) {
                200 -> return
                410 -> error("El código QR ya expiró o se usó. Genera uno nuevo.")
                404 -> error("El PC no tiene un pareo pendiente. Genera un QR nuevo.")
                else -> error("El PC respondió HTTP $code al vincular.")
            }
        } finally {
            connection.disconnect()
        }
    }

    private fun fetchV2(pairing: PairingInfo, clientDeviceId: String, secret: ByteArray): Pair<SyncPayload, String> {
        val baseUrl = "http://${bracketedHost(pairing.host)}:${pairing.port}"
        val meta = ServerMeta.fromJson(getJson("$baseUrl/v1/meta"))
        require(meta.schemaVersion == SUPPORTED_SCHEMA_VERSION && meta.blobVersion == SUPPORTED_BLOB_VERSION) { "El PC usa una versión de sync no compatible." }
        require(meta.lan) { "El servidor del PC no está expuesto a la red local." }
        require(meta.deviceId.equals(pairing.deviceId, true) && meta.fingerprint == pairing.fingerprint) { "El PC no coincide con el QR. No continúes la vinculación." }
        val blob = EncryptedBlob.fromJson(getJson("$baseUrl/v2/snapshot?device=$clientDeviceId"))
        val raw = crypto.decrypt(blob, secret)
        val payload = SyncPayload.fromJson(JSONObject(raw))
        require(payload.deviceId.equals(pairing.deviceId, true)) { "El snapshot no pertenece al PC vinculado." }
        return payload to raw
    }

    /** `Build.MODEL` crosses HTTP and gets stored/rendered on Desktop —
     * mirror the same sanitization rules as the Rust side. */
    private fun sanitizedDeviceName(): String {
        val cleaned = android.os.Build.MODEL.filterNot { it.isISOControl() }.trim()
        return if (cleaned.isEmpty()) "Android" else cleaned.take(64)
    }

    private fun fetch(pairing: PairingInfo, passphrase: CharArray): Pair<SyncPayload, String> {
        val baseUrl = "http://${bracketedHost(pairing.host)}:${pairing.port}"
        val meta = ServerMeta.fromJson(getJson("$baseUrl/v1/meta"))
        require(meta.schemaVersion == SUPPORTED_SCHEMA_VERSION && meta.blobVersion == SUPPORTED_BLOB_VERSION) { "El PC usa una versión de sync no compatible." }
        require(meta.lan) { "El servidor del PC no está expuesto a la red local." }
        require(meta.deviceId.equals(pairing.deviceId, true) && meta.fingerprint == pairing.fingerprint) { "El PC no coincide con el QR. No continúes la vinculación." }
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
