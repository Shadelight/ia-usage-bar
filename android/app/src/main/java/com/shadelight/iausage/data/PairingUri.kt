package com.shadelight.iausage.data

import java.security.MessageDigest
import java.net.URI
import java.net.URLDecoder

object PairingUri {
    private const val PREFIX = "iausage://pair"
    private const val CROCKFORD = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"

    fun parse(raw: String): PairingInfo {
        val trimmed = raw.trim()
        val uri = runCatching { URI(trimmed) }.getOrElse { throw IllegalArgumentException("El QR no es un enlace de IA Usage.") }
        require("iausage" == uri.scheme && "pair" == uri.host && trimmed.startsWith(PREFIX)) { "El QR no es un enlace de IA Usage." }
        val query = uri.rawQuery.orEmpty()
            .split('&')
            .mapNotNull { entry ->
                val separator = entry.indexOf('=')
                if (separator <= 0) null else {
                    val key = URLDecoder.decode(entry.substring(0, separator), "UTF-8")
                    val value = URLDecoder.decode(entry.substring(separator + 1), "UTF-8")
                    key to value
                }
            }
            .toMap()
        return when (query["v"]) {
            "1" -> parseV1(query)
            "2" -> parseV2(query)
            else -> throw IllegalArgumentException("La versión del QR no es compatible.")
        }
    }

    private fun parseV1(query: Map<String, String>): PairingInfo {
        val host = query["host"]?.trim().orEmpty()
        val port = query["port"]?.toIntOrNull()
        val device = query["device"]?.trim().orEmpty()
        val fingerprint = query["fp"]?.trim().orEmpty()
        val minApp = query["minApp"]?.trim().orEmpty()
        require(host.isNotEmpty() && !host.equals("localhost", true) && host != "127.0.0.1") { "El QR no contiene una dirección LAN válida." }
        require(port != null && port in 1..65535) { "El puerto del QR no es válido." }
        require(device.matches(Regex("[0-9a-fA-F]{16,128}"))) { "El identificador del PC no es válido." }
        require(fingerprint.matches(Regex("[0-9A-HJKMNPQRSTVWXYZ]{4}-[0-9A-HJKMNPQRSTVWXYZ]{4}"))) { "El código de verificación del QR no es válido." }
        require(fingerprint == fingerprintFor(device)) { "El código de verificación no corresponde al PC del QR." }
        require(minApp.matches(Regex("\\d+\\.\\d+\\.\\d+"))) { "La versión mínima del PC no es válida." }
        return PairingInfo(host, port, device.lowercase(), fingerprint, minApp)
    }

    private fun parseV2(query: Map<String, String>): PairingInfo {
        val host = query["host"]?.trim().orEmpty()
        val port = query["port"]?.toIntOrNull()
        val pc = query["pc"]?.trim().orEmpty()
        val fingerprint = query["fp"]?.trim().orEmpty()
        val token = query["token"]?.trim().orEmpty()
        val secretB64 = query["secret"]?.trim().orEmpty()
        require(host.isNotEmpty() && !host.equals("localhost", true) && host != "127.0.0.1") { "El QR no contiene una dirección LAN válida." }
        require(port != null && port in 1..65535) { "El puerto del QR no es válido." }
        require(pc.matches(Regex("[0-9a-fA-F]{16,128}"))) { "El identificador del PC no es válido." }
        require(fingerprint.matches(Regex("[0-9A-HJKMNPQRSTVWXYZ]{4}-[0-9A-HJKMNPQRSTVWXYZ]{4}"))) { "El código de verificación del QR no es válido." }
        require(fingerprint == fingerprintFor(pc)) { "El código de verificación no corresponde al PC del QR." }
        require(token.isNotEmpty()) { "El QR no contiene un token de pareo." }
        require(secretB64.isNotEmpty()) { "El QR no contiene un secreto de pareo." }
        // java.util.Base64 (not android.util.Base64) on purpose: the latter is an unmocked
        // Android stub under plain testDebugUnitTest (no Robolectric configured for this
        // module) and throws "not mocked" at runtime. java.util.Base64 is real JDK, works
        // under the JVM test runner, and has shipped on-device since API 26 — this app's
        // minSdk — so behavior is identical in tests and on real devices. Its URL decoder
        // accepts both padded and unpadded input, matching the wire format's unpadded output.
        val secret = runCatching {
            java.util.Base64.getUrlDecoder().decode(secretB64)
        }.getOrElse { throw IllegalArgumentException("El secreto del QR no es válido.") }
        // v2 URIs carry no minApp param (the min-version gate proved unnecessary friction for a
        // same-repo PC/phone pair by the time v2 was designed); "0.0.0" is a placeholder that
        // UsageSyncRepository's versionAtLeast check will trivially satisfy.
        return PairingInfo(host, port, pc.lowercase(), fingerprint, "0.0.0", token, secret)
    }

    fun fingerprintFor(deviceId: String): String {
        val digest = MessageDigest.getInstance("SHA-256").digest("iausage-pairing-v1:$deviceId".toByteArray())
        var bits = 0L
        digest.take(5).forEach { byte -> bits = (bits shl 8) or (byte.toLong() and 0xff) }
        val output = buildString { repeat(8) { i -> append(CROCKFORD[((bits shr (5 * (7 - i))) and 31).toInt()]) } }
        return output.take(4) + "-" + output.drop(4)
    }
}
