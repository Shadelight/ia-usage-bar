package com.shadelight.iausage.data

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

/** Golden vector shared with crates/iausage-core/src/sync.rs. Run on a real Android runtime. */
@RunWith(AndroidJUnit4::class)
class ProtocolCryptoInstrumentedTest {
    @Test fun opensTheM5GoldenVector() {
        val blob = EncryptedBlob.fromJson(JSONObject("""{
          "v":1,"alg":"xchacha20poly1305+argon2id",
          "salt":"AQIDBAUGBwgJCgsMDQ4PEA==",
          "nonce":"ERERERERERERERERERERERERERERERER",
          "ciphertext":"CnwT2OjdYeHwxCsDnU1POJiWQLHsJ4zdZ8IRClCR+DHuldB2zyvD+5NZ5xoWTDEbdo3pCmTFNihgXsdS+ylwdiuC90VqSJ+3yRmjkY8w5KA9cXkgVDATeD5iC0+Plz0UQ4WR8LySM6AT0nC+cxjff1m6hZwVWdoWPsuZ63sX9BIf0RwL9IqlxVk0YeRPgueCpm2/UjUvpe5ZcMcCTiDQxKv/J/BnIIlJtBnvB8o/AQ7TjkF7UG57ChfJzswWBiCJYvwTiVb4yJRcQc8M"
        }"""))
        val payload = SyncPayload.fromJson(JSONObject(ProtocolCrypto().decrypt(blob, "vector-test-passphrase".toCharArray())))
        assertEquals("vector-device", payload.deviceId)
        assertEquals(1, payload.schemaVersion)
    }
}
