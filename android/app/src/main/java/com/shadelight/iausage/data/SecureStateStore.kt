package com.shadelight.iausage.data

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Private storage encrypted by an AES key that never leaves Android Keystore. */
class SecureStateStore(context: Context) {
    private val preferences = context.getSharedPreferences("iausage.secure.v1", Context.MODE_PRIVATE)

    fun savePairing(pairing: PairingInfo, passphrase: CharArray) {
        write("pairing", listOf(pairing.host, pairing.port, pairing.deviceId, pairing.fingerprint, pairing.minAppVersion).joinToString("\n"))
        write("passphrase", passphrase.concatToString())
        passphrase.fill('\u0000')
    }

    fun pairing(): PairingInfo? = read("pairing")?.split('\n')?.let { values ->
        if (values.size != 5) null else runCatching { PairingInfo(values[0], values[1].toInt(), values[2], values[3], values[4]) }.getOrNull()
    }

    fun passphrase(): CharArray? = read("passphrase")?.toCharArray()

    fun savePairingV2(pairing: PairingInfo, clientDeviceId: String, secret: ByteArray) {
        write("pairing", listOf(pairing.host, pairing.port, pairing.deviceId, pairing.fingerprint, pairing.minAppVersion).joinToString("\n"))
        write("deviceSecret", Base64.encodeToString(secret, Base64.NO_WRAP))
        preferences.edit().remove("passphrase").apply()
        secret.fill(0)
    }

    fun deviceSecret(): ByteArray? = read("deviceSecret")?.let { Base64.decode(it, Base64.NO_WRAP) }

    fun clientDeviceId(): String? = read("clientDeviceId")

    fun ensureClientDeviceId(): String {
        read("clientDeviceId")?.let { return it }
        val bytes = ByteArray(16)
        java.security.SecureRandom().nextBytes(bytes)
        val id = bytes.joinToString("") { "%02x".format(it) }
        write("clientDeviceId", id)
        return id
    }

    fun savePayload(rawPayload: String) = write("payload", rawPayload)
    fun payload(): String? = read("payload")
    /** Drops only the cached snapshot — pairing/passphrase survive. Distinct
     * from `clear()`: "clear local data" in Settings must not silently
     * unlink the PC the way "Disconnect" does. */
    fun clearPayload() = preferences.edit().remove("payload").apply()

    /** `clientDeviceId` survives: re-pairing this same phone after a
     * disconnect must still be recognized server-side as the same device
     * (an upsert, not a fresh row) — see the pairing V2 spec's "Re-pairing"
     * section. Everything else (pairing, passphrase, deviceSecret, payload)
     * is wiped as before. */
    fun clear() {
        val preservedClientId = read("clientDeviceId")
        preferences.edit().clear().apply()
        preservedClientId?.let { write("clientDeviceId", it) }
    }

    private fun write(name: String, cleartext: String) {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.ENCRYPT_MODE, key()) }
        val packed = cipher.iv + cipher.doFinal(cleartext.toByteArray(Charsets.UTF_8))
        preferences.edit().putString(name, Base64.encodeToString(packed, Base64.NO_WRAP)).apply()
    }

    private fun read(name: String): String? = runCatching {
        val packed = Base64.decode(preferences.getString(name, null) ?: return null, Base64.NO_WRAP)
        require(packed.size > 12) { "Estado local inválido" }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, packed.copyOfRange(0, 12))) }
        String(cipher.doFinal(packed.copyOfRange(12, packed.size)), Charsets.UTF_8)
    }.getOrNull()

    private fun key(): SecretKey {
        val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (keyStore.getKey(KEY_ALIAS, null) as? SecretKey)?.let { return it }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder(KEY_ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .build())
        }.generateKey()
    }

    private companion object { const val KEY_ALIAS = "iausage.mobile.storage.v1" }
}
