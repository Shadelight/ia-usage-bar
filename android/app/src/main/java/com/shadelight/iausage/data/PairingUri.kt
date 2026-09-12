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
        require(query["v"] == "1") { "La versión del QR no es compatible." }
        val host = query["host"]?.trim().orEmpty()
        val port = query["port"]?.toIntOrNull()
        val device = query["device"]?.trim().orEmpty()
        val fingerprint = query["fp"]?.trim().orEmpty()
        val minApp = query["minApp"]?.trim().orEmpty()
        require(host.isNotEmpty() && !host.equals("localhost", true) && host != "127.0.0.1") { "El QR no contiene una dirección LAN válida." }
        require(port != null && port in 1..65535) { "El puerto del QR no es válido." }
        require(device.matches(Regex("[0-9a-fA-F]{16,128}"))) { "El identificador del PC no es válido." }
        require(fingerprint.matches(Regex("[0-9A-HJKMNPQRSTVWXYZ]{4}-[0-9A-HJKMNPQRSTVWXYZ]{4}"))) { "El fingerprint del QR no es válido." }
        require(fingerprint == fingerprintFor(device)) { "El fingerprint no corresponde al PC del QR." }
        require(minApp.matches(Regex("\\d+\\.\\d+\\.\\d+"))) { "La versión mínima del PC no es válida." }
        return PairingInfo(host, port, device.lowercase(), fingerprint, minApp)
    }

    fun fingerprintFor(deviceId: String): String {
        val digest = MessageDigest.getInstance("SHA-256").digest("iausage-pairing-v1:$deviceId".toByteArray())
        var bits = 0L
        digest.take(5).forEach { byte -> bits = (bits shl 8) or (byte.toLong() and 0xff) }
        val output = buildString { repeat(8) { i -> append(CROCKFORD[((bits shr (5 * (7 - i))) and 31).toInt()]) } }
        return output.take(4) + "-" + output.drop(4)
    }
}
