package com.shadelight.iausage

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class DeepLinkTest {
    @Test fun `pair deep link is delivered unchanged`() {
        val uri = "iausage://pair?v=1&host=192.168.1.15"
        assertEquals(uri, acceptedPairingUri(uri))
    }

    @Test fun `other schemes and hosts are ignored`() {
        assertNull(acceptedPairingUri("https://pair?v=1"))
        assertNull(acceptedPairingUri("iausage://settings?v=1"))
        assertNull(acceptedPairingUri(null))
    }
}
