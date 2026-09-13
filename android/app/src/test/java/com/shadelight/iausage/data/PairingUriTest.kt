package com.shadelight.iausage.data

import org.junit.Assert.assertEquals
import org.junit.Test

class PairingUriTest {
    @Test fun `parses the M5 pairing URI`() {
        val device = "0123456789abcdef0123456789abcdef"
        val fp = PairingUri.fingerprintFor(device)
        val result = PairingUri.parse("iausage://pair?v=1&host=192.168.1.15&port=28741&device=$device&fp=$fp&minApp=0.3.0")
        assertEquals("192.168.1.15", result.host)
        assertEquals(fp, result.fingerprint)
    }

    @Test(expected = IllegalArgumentException::class) fun `rejects tampered fingerprint`() {
        PairingUri.parse("iausage://pair?v=1&host=192.168.1.15&port=28741&device=0123456789abcdef&fp=AAAA-AAAA&minApp=0.3.0")
    }

    @Test(expected = IllegalArgumentException::class) fun `rejects a different deep link host`() {
        val device = "0123456789abcdef0123456789abcdef"
        val fp = PairingUri.fingerprintFor(device)
        PairingUri.parse("iausage://settings?v=1&host=192.168.1.15&port=28741&device=$device&fp=$fp&minApp=0.2.7")
    }
}
