package com.shadelight.iausage.data

import android.content.Context
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest

private const val LATEST_URL = "https://api.github.com/repos/Shadelight/ia-usage-bar/releases/latest"
private const val USER_AGENT = "IA-Usage-Bar-Android"

/**
 * Cliente de auto-actualización contra GitHub Releases. Mismo trust model que
 * Windows: release → APK → `SHA256SUMS.txt` de ESA release → SHA-256 local →
 * instalador del sistema. Cualquier fallo deja la app intacta: nunca cierra,
 * nunca borra pairing ni datos; el llamador lo mapea a `UpdateState.Failed`.
 *
 * La descarga escribe `<apk>.part` y solo tras verificar se renombra a `.apk`.
 * El repositorio no conoce estados de UI: devuelve datos o lanza con mensaje
 * en español listo para mostrar.
 */
class GithubUpdateRepository(context: Context) {
    private val updatesDir: File = File(context.applicationContext.cacheDir, "updates").apply { mkdirs() }

    sealed interface CheckResult {
        data object UpToDate : CheckResult
        data class Available(val release: ReleaseUpdate) : CheckResult
    }

    fun check(currentVersion: String): CheckResult {
        val release = parseLatestRelease(getText(LATEST_URL, "No se pudo consultar la última versión."))
            ?: throw IllegalStateException("La versión publicada no incluye APK ni checksum.")
        return if (compareVersions(release.version, currentVersion) > 0) {
            CheckResult.Available(release.copy(apkSize = null))
        } else {
            CheckResult.UpToDate
        }
    }

    /**
     * Descarga el APK de `release` (metadata ya resuelta: no se reconsulta
     * `/releases/latest` aquí) a `<apk>.part`. `onProgress` recibe 0..1 o
     * null si GitHub no entregó `Content-Length`.
     */
    fun download(release: ReleaseUpdate, onProgress: (Float?) -> Unit): File {
        val part = File(updatesDir, "${release.apkName}.part")
        val connection = openGet(release.apkUrl)
        try {
            val code = connection.responseCode
            if (code !in 200..299) throw IllegalStateException("La descarga falló con HTTP $code.")
            val total = connection.contentLengthLong.takeIf { it > 0 }
            onProgress(if (total != null) 0f else null)
            connection.inputStream.use { input ->
                part.outputStream().use { output ->
                    val buffer = ByteArray(64 * 1024)
                    var read: Int
                    var done = 0L
                    while (input.read(buffer).also { read = it } != -1) {
                        output.write(buffer, 0, read)
                        done += read
                        if (total != null) onProgress((done.toFloat() / total).coerceIn(0f, 1f))
                    }
                }
            }
        } catch (e: Exception) {
            part.delete()
            throw e as? IllegalStateException ?: IllegalStateException("No se pudo descargar la actualización.")
        } finally {
            connection.disconnect()
        }
        return part
    }

    /** Verifica el `.part` contra el `SHA256SUMS.txt` de ESA misma release. */
    fun verify(release: ReleaseUpdate, part: File) {
        val body = getText(release.checksumUrl, "No se pudo descargar SHA256SUMS.txt.")
        val expected = parseChecksum(body, release.apkName)
            ?: throw IllegalStateException("No hay checksum publicado para ${release.apkName}.")
        val actual = sha256Hex(part)
        if (!actual.equals(expected, ignoreCase = true)) {
            part.delete()
            throw IllegalStateException("La descarga no coincide con el checksum publicado.")
        }
    }

    /** Renombra `<apk>.part` → `<apk>` tras verificar. Limpia restos viejos. */
    fun promote(part: File, apkName: String): File {
        val apk = File(updatesDir, apkName)
        updatesDir.listFiles()?.forEach { file ->
            if (file != part && file != apk && (file.name.endsWith(".part") || file.name.endsWith(".apk"))) {
                file.delete()
            }
        }
        if (apk.exists()) apk.delete()
        if (!part.renameTo(apk)) throw IllegalStateException("No se pudo preparar el instalador.")
        return apk
    }

    fun cachedApk(apkName: String): File? =
        File(updatesDir, apkName).takeIf { it.isFile && it.length() > 0 }

    private fun getText(url: String, errorMessage: String): String {
        val connection = openGet(url)
        return try {
            val code = connection.responseCode
            val body = (if (code in 200..299) connection.inputStream else connection.errorStream)
                ?.bufferedReader()?.use { it.readText() }.orEmpty()
            if (code !in 200..299) throw IllegalStateException(errorMessage)
            body
        } catch (e: IllegalStateException) {
            throw e
        } catch (e: Exception) {
            throw IllegalStateException(errorMessage)
        } finally {
            connection.disconnect()
        }
    }

    private fun openGet(url: String): HttpURLConnection =
        (URL(url).openConnection() as HttpURLConnection).apply {
            connectTimeout = 8_000
            readTimeout = 20_000
            requestMethod = "GET"
            setRequestProperty("User-Agent", USER_AGENT)
            setRequestProperty("Accept", "application/vnd.github+json")
            instanceFollowRedirects = true
        }

    private fun sha256Hex(file: File): String {
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { input ->
            val buffer = ByteArray(64 * 1024)
            var read: Int
            while (input.read(buffer).also { read = it } != -1) {
                digest.update(buffer, 0, read)
            }
        }
        return digest.digest().joinToString("") { "%02x".format(it) }
    }
}
