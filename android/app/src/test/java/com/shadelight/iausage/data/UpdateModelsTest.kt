package com.shadelight.iausage.data

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

private fun releaseJson(
    tag: String = "v0.4.0",
    htmlUrl: String = "https://github.com/Shadelight/ia-usage-bar/releases/tag/v0.4.0",
    assets: String = """
        {"name": "IA-Usage-Android-0.4.0.apk", "browser_download_url": "https://github.com/dl/app.apk"},
        {"name": "IA-Usage-Android-0.4.0.aab", "browser_download_url": "https://github.com/dl/app.aab"},
        {"name": "SHA256SUMS.txt", "browser_download_url": "https://github.com/dl/sums"}
    """.trimIndent(),
) = """{"tag_name": "$tag", "html_url": "$htmlUrl", "assets": [$assets]}"""

class UpdateModelsTest {
    @Test fun `release resolves apk and checksum urls from the same payload`() {
        val release = parseLatestRelease(releaseJson())!!
        assertEquals("v0.4.0", release.tag)
        assertEquals("0.4.0", release.version)
        assertEquals("IA-Usage-Android-0.4.0.apk", release.apkName)
        assertEquals("https://github.com/dl/app.apk", release.apkUrl)
        assertEquals("https://github.com/dl/sums", release.checksumUrl)
    }

    @Test fun `release without apk is unusable`() {
        val noApk = releaseJson(assets = """{"name": "SHA256SUMS.txt", "browser_download_url": "https://github.com/dl/sums"}""")
        assertNull(parseLatestRelease(noApk))
    }

    @Test fun `release without checksum file is unusable`() {
        val noSums = releaseJson(assets = """{"name": "IA-Usage-Android-0.4.0.apk", "browser_download_url": "https://github.com/dl/app.apk"}""")
        assertNull(parseLatestRelease(noSums))
    }

    @Test fun `malformed release json is unusable, never crashing`() {
        assertNull(parseLatestRelease("not json at all"))
        assertNull(parseLatestRelease("{}"))
        assertNull(parseLatestRelease("""{"tag_name": "", "html_url": "", "assets": []}"""))
    }

    @Test fun `version comparison matches Windows version_parts semantics`() {
        assertEquals(0, compareVersions("0.4.0", "0.4.0"))
        assertEquals(0, compareVersions("v0.4.0", "0.4.0")) // leading v ignored
        assertTrue(compareVersions("0.4.0", "0.3.9") > 0)
        assertTrue(compareVersions("0.3.0", "0.4.0") < 0)
        assertTrue(compareVersions("0.10.0", "0.9.9") > 0) // numeric, not lexicographic
        assertEquals(0, compareVersions("0.4.0-beta", "0.4.0")) // suffix cut at '-'
        assertEquals(0, compareVersions("0.4.x", "0.4.0")) // invalid segment -> 0
        assertTrue(compareVersions("1.0", "0.9.9") > 0)
    }

    @Test fun `checksum parses plain and binary-mode lines`() {
        val hash = "a".repeat(64)
        val body = "$hash  IA-Usage-Android-0.4.0.apk\n${"b".repeat(64)} *other.apk\n"
        assertEquals(hash, parseChecksum(body, "IA-Usage-Android-0.4.0.apk"))
    }

    @Test fun `checksum rejects short or non-hex hashes and unknown names`() {
        assertNull(parseChecksum("abc123  IA-Usage-Android-0.4.0.apk\n", "IA-Usage-Android-0.4.0.apk"))
        assertNull(parseChecksum("${"g".repeat(64)}  IA-Usage-Android-0.4.0.apk\n", "IA-Usage-Android-0.4.0.apk"))
        assertNull(parseChecksum("${"a".repeat(64)}  other.apk\n", "IA-Usage-Android-0.4.0.apk"))
        assertNull(parseChecksum("", "IA-Usage-Android-0.4.0.apk"))
    }
}
