package com.shadelight.iausage.data

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class SecureStateStoreInstrumentedTest {
    private fun freshStore(): SecureStateStore {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        context.getSharedPreferences("iausage.secure.v1", android.content.Context.MODE_PRIVATE).edit().clear().apply()
        return SecureStateStore(context)
    }

    @Test fun clientDeviceId_is_generated_once_and_persists() {
        val store = freshStore()
        val id = store.ensureClientDeviceId()
        assertEquals(id, store.ensureClientDeviceId())
        assertEquals(id, store.clientDeviceId())
    }

    @Test fun deviceSecret_round_trips() {
        val store = freshStore()
        val pairing = PairingInfo("192.168.50.116", 28741, "pc1", "PYYZ-JMJJ", "0.0.0")
        val secret = ByteArray(32) { it.toByte() }
        // savePairingV2 zeroes the passed-in array in place after persisting it
        // (matching this class's passphrase.fill() pattern), so compare against
        // a copy taken before the call rather than the now-wiped `secret`.
        val secretCopy = secret.copyOf()
        store.savePairingV2(pairing, "phone-1", secret)
        assertArrayEquals(secretCopy, store.deviceSecret())
        assertEquals(pairing.host, store.pairing()?.host)
    }

    @Test fun clear_preserves_clientDeviceId_but_wipes_everything_else() {
        val store = freshStore()
        val id = store.ensureClientDeviceId()
        val pairing = PairingInfo("192.168.50.116", 28741, "pc1", "PYYZ-JMJJ", "0.0.0")
        store.savePairingV2(pairing, "phone-1", ByteArray(32))
        store.clear()
        assertEquals(id, store.clientDeviceId())
        assertNull(store.deviceSecret())
        assertNull(store.pairing())
    }
}
