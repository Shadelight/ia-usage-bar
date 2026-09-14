package com.shadelight.iausage.data

import org.json.JSONObject

/**
 * Metadata inmutable de UNA release concreta. Se resuelve una sola vez en
 * `check()` y todo el flujo (descarga + verificación) consume este objeto:
 * nunca se vuelve a consultar `/releases/latest` a mitad de camino, porque
 * podría haber aparecido otra release entre medias.
 */
data class ReleaseUpdate(
    /** Tag tal cual viene en `tag_name`, p. ej. "v0.4.0". */
    val tag: String,
    /** Versión sin la `v`, p. ej. "0.4.0". */
    val version: String,
    val apkName: String,
    val apkUrl: String,
    val apkSize: Long?,
    /** SHA256SUMS.txt de ESA misma release. */
    val checksumUrl: String,
    /** URL humana de la release ("Ver cambios"). */
    val releaseUrl: String,
)

sealed interface UpdateState {
    data object Idle : UpdateState
    data object Checking : UpdateState
    data object UpToDate : UpdateState
    data class Available(val release: ReleaseUpdate) : UpdateState
    /** `progress` null = GitHub no entregó Content-Length: barra indeterminada. */
    data class Downloading(val release: ReleaseUpdate, val progress: Float?) : UpdateState
    data class Verifying(val release: ReleaseUpdate) : UpdateState
    data class ReadyToInstall(val release: ReleaseUpdate, val fileName: String) : UpdateState
    data class Failed(val reason: String) : UpdateState
}

/**
 * Paridad exacta con `version_parts` de Windows (`commands.rs`): trim de `v`,
 * split por `.`, corte en `-`, segmento inválido → 0, comparación
 * lexicográfica numérica (incluido que `[0,4] > [0,3,9]`).
 */
fun compareVersions(a: String, b: String): Int {
    fun parts(value: String): List<Long> =
        value.trimStart('v').split('.').map { part ->
            part.substringBefore('-').toLongOrNull() ?: 0L
        }
    val pa = parts(a)
    val pb = parts(b)
    val n = maxOf(pa.size, pb.size)
    for (i in 0 until n) {
        val diff = (pa.getOrElse(i) { 0L }).compareTo(pb.getOrElse(i) { 0L })
        if (diff != 0) return diff
    }
    return pa.size.compareTo(pb.size)
}

/**
 * Extrae el hash SHA-256 publicado para `assetName` del contenido de
 * `SHA256SUMS.txt`. Misma tolerancia que Windows: prefijo `*` de modo
 * binario, 64 dígitos hex. Función pura.
 */
fun parseChecksum(body: String, assetName: String): String? {
    // GitHub sustituye espacios por puntos al subir assets; el checksum ya
    // viene con el nombre con puntos, igual que la API.
    return body.lines()
        .mapNotNull { line ->
            val fields = line.trim().split(Regex("\\s+"))
            if (fields.size < 2) return@mapNotNull null
            val hash = fields[0].lowercase()
            val name = fields[1].trimStart('*')
            if (name == assetName) hash else null
        }
        .firstOrNull()
        ?.takeIf { it.length == 64 && it.all { byte -> byte.isDigit() || byte in 'a'..'f' } }
}

/**
 * Resuelve `ReleaseUpdate` desde el JSON de `releases/latest`. Devuelve null
 * si falta el APK o el SHA256SUMS.txt: sin checksum publicado no se instala
 * nada, igual que en Windows.
 */
fun parseLatestRelease(body: String): ReleaseUpdate? {
    val root = runCatching { JSONObject(body) }.getOrNull() ?: return null
    val tag = root.optString("tag_name").takeIf { it.isNotBlank() } ?: return null
    val releaseUrl = root.optString("html_url").takeIf { it.isNotBlank() } ?: return null
    val assets = root.optJSONArray("assets") ?: return null
    var apkName: String? = null
    var apkUrl: String? = null
    var checksumUrl: String? = null
    for (i in 0 until assets.length()) {
        val asset = assets.optJSONObject(i) ?: continue
        val name = asset.optString("name")
        val url = asset.optString("browser_download_url")
        if (name.isBlank() || url.isBlank()) continue
        if (name.endsWith(".apk", ignoreCase = true) && apkUrl == null) {
            apkName = name
            apkUrl = url
        }
        if (name.equals("SHA256SUMS.txt", ignoreCase = true) && checksumUrl == null) {
            checksumUrl = url
        }
    }
    if (apkName == null || apkUrl == null || checksumUrl == null) return null
    return ReleaseUpdate(
        tag = tag,
        version = tag.trimStart('v'),
        apkName = apkName,
        apkUrl = apkUrl,
        apkSize = null,
        checksumUrl = checksumUrl,
        releaseUrl = releaseUrl,
    )
}
