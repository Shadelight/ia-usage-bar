package com.shadelight.iausage.data

import android.util.Base64
import com.goterl.lazysodium.LazySodiumAndroid
import com.goterl.lazysodium.SodiumAndroid
import com.goterl.lazysodium.interfaces.AEAD
import com.goterl.lazysodium.utils.Key
import org.bouncycastle.crypto.generators.Argon2BytesGenerator
import org.bouncycastle.crypto.params.Argon2Parameters

/** Exact M5 wire decoder: Argon2id m=64MiB, t=3, p=4 then XChaCha20-Poly1305-IETF. */
class ProtocolCrypto {
    private val sodium = LazySodiumAndroid(SodiumAndroid())

    fun decrypt(blob: EncryptedBlob, passphrase: CharArray): String {
        require(blob.version == SUPPORTED_BLOB_VERSION) { "La versión del blob no es compatible." }
        require(blob.algorithm == SUPPORTED_ALGORITHM) { "El algoritmo del blob no es compatible." }
        require(passphrase.isNotEmpty()) { "La passphrase es obligatoria." }
        val salt = decode(blob.salt, "salt", 16)
        val nonce = decode(blob.nonce, "nonce", 24)
        val ciphertext = decode(blob.ciphertext, "ciphertext", null)
        return try {
            val key = deriveKey(passphrase, salt)
            sodium.decrypt(
                sodium.toHexStr(ciphertext),
                null,
                nonce,
                Key.fromBytes(key),
                AEAD.Method.XCHACHA20_POLY1305_IETF,
            ) ?: throw SecurityException("No se pudo descifrar: revisa la passphrase.")
        } finally {
            passphrase.fill('\u0000')
        }
    }

    private fun deriveKey(passphrase: CharArray, salt: ByteArray): ByteArray {
        // Unlike libsodium's high-level crypto_pwhash API, Bouncy Castle lets
        // us pin the M5 parallelism value (p = 4) as well as memory and time.
        val parameters = Argon2Parameters.Builder(Argon2Parameters.ARGON2_id)
            .withVersion(Argon2Parameters.ARGON2_VERSION_13)
            .withSalt(salt)
            .withMemoryAsKB(64 * 1024)
            .withIterations(3)
            .withParallelism(4)
            .build()
        return ByteArray(AEAD.XCHACHA20POLY1305_IETF_KEYBYTES).also { output ->
            Argon2BytesGenerator().apply { init(parameters) }.generateBytes(passphrase, output)
        }
    }

    private fun decode(value: String, field: String, expectedSize: Int?): ByteArray = try {
        Base64.decode(value, Base64.DEFAULT).also { bytes ->
            require(expectedSize == null || bytes.size == expectedSize) { "$field tiene una longitud inválida." }
        }
    } catch (error: IllegalArgumentException) {
        throw IllegalArgumentException("$field no es base64 válido.", error)
    }
}
