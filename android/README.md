# IA Usage para Android

El módulo `android/` es el visor móvil de IA Usage. No contiene claves de
proveedores, cookies ni sesiones: solo obtiene y abre localmente el blob M5
que el PC ya cifra.

## Ejecutar

Abre la carpeta `android/` en Android Studio (JDK 21) y ejecuta la variante
`debug` en un dispositivo con Android 8.0 o superior. También se puede usar
una instalación local de Gradle 9.1:

```powershell
cd android
gradle :app:testDebugUnitTest :app:assembleDebug
```

En Windows activa **Ajustes → Teléfono y sync → Exponer en la red local**,
guarda una passphrase y muestra el QR. En el teléfono escanéalo, comprueba el
fingerprint y escribe la misma passphrase. El tráfico HTTP solo transporta el
envelope cifrado; la app valida `meta`, descifra el snapshot en el dispositivo
y almacena el último snapshot cifrado localmente, protegiendo su clave
mediante Android Keystore.

## Notas M6

- **HTTP LAN:** el PC expone `http://<ip-lan>:28741` sin TLS (red de
  confianza). Android lo permite vía `usesCleartextTraffic` +
  `res/xml/network_security_config.xml`. El blob ya viaja cifrado
  (XChaCha20-Poly1305 + Argon2id); un atacante de red solo ve metadatos.
- **QR:** se usa el escáner de Google Play
  (`play-services-code-scanner`), que no requiere permiso `CAMERA` en el
  manifiesto. `PairingUri.parse` rechaza esquema/version (`v=1`) inválidos,
  hosts no-LAN (`localhost`/`127.0.0.1`), puertos fuera de rango y
  fingerprints que no derivan del `deviceId`. `UsageSyncRepository` vuelve a
  validar `schemaVersion`/`blobVersion`, `deviceId` + `fingerprint` contra
  `/v1/meta` y `minApp` antes de descifrar.
- **Keystore:** nada queda en preferencias planas. `SecureStateStore` cifra
  pairing + passphrase + snapshot con AES/GCM cuya clave (`alias
  iausage.mobile.storage.v1`) nunca sale de Android Keystore. Esquema:

  ```text
  Android Keystore
  └── clave de cifrado / wrapping key (alias iausage.mobile.storage.v1)

  almacenamiento privado de la app
  └── snapshot cifrado (AES/GCM + Base64 en SharedPreferences privadas)
  ```

## Publicar APK + AAB firmados

El workflow de tags necesita estos secretos de GitHub. No se han creado ni
incluido claves de firma en este repositorio:

| Secreto | Valor |
|---|---|
| `ANDROID_KEYSTORE_BASE64` | keystore de release, codificado en base64 |
| `ANDROID_STORE_PASSWORD` | contraseña del keystore |
| `ANDROID_KEY_ALIAS` | alias de la clave |
| `ANDROID_KEY_PASSWORD` | contraseña de la clave |

Al publicar una etiqueta `v*`, el job `android` ejecuta
`./gradlew :app:assembleRelease :app:bundleRelease` y el job `release`
(Windows, el único que crea la GitHub Release vía `tauri-action`) sube los
assets a esa misma release — Android nunca crea una segunda release `v*`.
Layout resultante:

```text
Git tag v0.3.0
        │
        ├── Windows
        │   ├── .exe
        │   └── .msi
        │
        └── Android
            ├── ia-usage-android-release.apk   # distribución directa
            └── ia-usage-android-release.aab   # Google Play (cuando se publique)
```

Más `SHA256SUMS.txt` (cubre `.exe`, `.msi`, `.apk`, `.aab`, `.vsix`).

- **Versionado:** `versionName` sigue el tag (`v0.3.0` → `0.3.0`,
  `-PIAUSAGE_VERSION_NAME`). `versionCode` se inyecta en CI con
  `-PIAUSAGE_VERSION_CODE=$GITHUB_RUN_NUMBER` para que crezca siempre
  aunque `versionName` se repita. El default local (`3` para `0.3.0`) debe
  subirse a mano si quedase por debajo del último `versionCode` de Play.
- El APK es para distribución directa; el AAB se genera desde ya para no
  tener que cambiar el pipeline el día que se publique en Play Console
  (ficha + privacidad + cuenta del propietario, fuera de este repo).

## Widget (M7, ya esbozado)

El widget Glance lee el **snapshot local cacheado**, nunca llama al PC en
`provideGlance`. El refresco lo hace `WorkManager` / la app (`RefreshWorker`
→ LAN → snapshot local → `updateAll`). Sin conexión muestra el último estado
con su edad (`Datos antiguos · hace … · PC sin conexión`) en vez de
quedarse vacío.
