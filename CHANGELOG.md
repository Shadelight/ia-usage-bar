# Changelog

All notable changes to this project are documented here. This project follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.3.6] - 2026-09-14

### Fixed

- Antigravity: el botón "Iniciar sesión" de una sesión vencida solo daba el
  error "no tiene un inicio de sesión automático". Para proveedores sin login
  lanzable desde la app ahora ofrece "Reintentar" (tras renovar la sesión en
  su propia aplicación).
- Antigravity: la vía local buscaba el token CSRF (un PowerShell de ~2 s) una
  vez por cada puerto probado, lo que podía superar los 12 s del refresco y
  dejar en pantalla el error anterior aunque la app estuviera abierta. Ahora
  se busca una sola vez por refresco.
- Modo claro: varios colores estaban fijos para el tema oscuro. La "A" del
  logo desaparecía, el banner de recomendación y los avisos de ritmo
  (verde/amarillo claro) eran ilegibles, los porcentajes con acentos claros
  (ámbar, naranja) tenían poco contraste, y la pastilla de idioma, los
  botones, los hovers y el borde de "+" usaban grises oscuros. Ahora pasan
  por tokens con valor propio en cada tema; el modo oscuro no cambia.
- CLI en Ajustes: decía "No está en PATH" aunque el instalador lo había
  configurado, porque la ruta del recurso llega con el prefijo `\\?\` y no
  coincidía con la entrada de PATH. "Reparar PATH" además escribía esa ruta
  con `\\?\`. Ahora se compara y se escribe sin prefijo.
- SuperGrok: tras conectar no mostraba ningún dato porque la respuesta de
  facturación de xAI cambió a `{"config": {"used", "monthlyLimit", ...}}`.
  Ahora muestra el uso mensual cuando el plan tiene límite de pago por uso,
  o "Sin cargos este período" cuando no lo tiene.

## [0.3.5] - 2026-09-14

Desde 0.3.4 el botón "Instalar ahora" de Windows no funciona: esta versión
hay que instalarla a mano una vez (en Windows y en Android). Después de
instalarla, vuelve a vincular el teléfono con un QR nuevo.

### Changed

- CLI: `iausage best` usa el mismo motor de recomendación que el banner del
  dashboard (margen, sostenibilidad y proveedor actual) en vez del cálculo
  legado `most_headroom`, que se elimina.

### Fixed

- Build: se quita el `[profile.release]` duplicado de `src-tauri/Cargo.toml`
  (Cargo lo ignoraba; el perfil efectivo ya vive en la raíz del workspace).
- Windows: "Instalar ahora" cerraba la app sin instalar nada. El helper de
  PowerShell se lanzaba con `DETACHED_PROCESS` y terminaba antes de ejecutar
  el script; ahora se lanza sin ventana y fuera del Job de la app. Las
  versiones hasta 0.3.4 traen el helper roto: esta actualización hay que
  instalarla a mano una vez.
- Teléfono y sync: apagar y volver a encender "Exponer en la red local" daba
  os error 10048 y dejaba el servidor detenido y la opción apagada (el
  teléfono vinculado ya no alcanzaba el PC). El puerto se re-bindeaba antes
  de que el servidor anterior lo liberara; ahora `bind` reintenta un momento.
- Antigravity: con la app cerrada, un access token de Google vencido
  aparecía como "La sesión ya no es válida" (HTTP 401) e invitaba a iniciar
  sesión. Ahora se detecta el vencimiento antes de llamar a Google y se pide
  abrir Antigravity, que es quien renueva la sesión.
- Sesión OAuth vencida: el backend enviaba el motivo como `o_auth_expired` y
  la interfaz esperaba `oauth_expired`, así que nunca mostraba "La sesión de
  OAuth venció". Se corrige el nombre (los snapshots en caché con el nombre
  viejo se siguen leyendo).
- SuperGrok: la CLI de Grok guarda ahora la sesión anidada bajo su cuenta
  (`https://auth.x.ai::<id>`), así que se mostraba "Falta iniciar sesión"
  aunque había sesión. Se leen ambos formatos, se avisa si la sesión caducó
  y la pista indica la ruta de `grok.exe`, que no queda en el PATH.
- Recomendación: si el proveedor actual se agota antes de su reset, el banner
  recomienda la alternativa que aguanta claramente más (p. ej. Claude a punto
  de agotarse con Codex libre). Antes el margen de 15 puntos y el bonus del
  proveedor actual lo impedían y mostraba "sin alternativa mejor".
- Android: "Acerca de" (versión y buscar actualizaciones) solo aparecía con
  un PC vinculado; ahora está siempre en el menú y la versión se ve en la
  barra superior.
- Android: la primera vinculación V2 fallaba al descifrar el snapshot: el
  secreto se ponía a cero al guardarlo, antes de usarlo. El PC ya había
  registrado el teléfono, así que reintentar mostraba "El PC no tiene un
  pareo pendiente".

## [0.3.4] - 2026-09-14

Incluye todo lo de 0.3.3, cuyo release falló en CI y nunca llegó a los
usuarios.

### Changed

- Dashboard: las pestañas de proveedor son la única identidad del proveedor.
  La pestaña activa muestra nombre y plan; las demás se compactan a icono o
  código corto según el ancho, y ya no se repite el encabezado debajo.

### Added

- Vinculación de teléfono sin passphrase (Sync V2): "Vincular teléfono"
  genera un código QR de un solo uso con un secreto de 256 bits que el
  usuario nunca ve ni escribe. El teléfono lo escanea, confirma el código
  de verificación y queda vinculado — sin inventar ni copiar una frase
  secreta. Cada teléfono tiene su propio secreto y su propia fila en una
  nueva lista de "Dispositivos vinculados", con botón para desvincular.
  La vinculación V1 (passphrase manual) sigue funcionando sin cambios
  para quien ya la usaba.
- Android: comprobador de actualizaciones dentro de la app — revisa los
  releases de GitHub, muestra un aviso descartable y puede descargar,
  verificar e instalar el APK más nuevo a través del instalador del
  sistema.

### Fixed

- Las notificaciones de "cuota reiniciada" se repetían cada pocos
  minutos para proveedores con ventana deslizante (p. ej. OpenCode Go):
  cualquier salto hacia adelante en `resetAt` se interpretaba como un
  reset real, pero una ventana deslizante avanza `resetAt` en cada
  consulta sin que la cuota se vacíe. Ahora también exige que el uso
  haya bajado, no solo que el timestamp haya avanzado.
- Tests: que desvincular un teléfono borra su secreto se verifica en CI con
  un almacén de credenciales en memoria, sin depender del Credential Manager
  del runner Windows de GitHub (ahí `CredDeleteW` no borra de inmediato, lo
  que bloqueaba el release). El smoke test contra el Credential Manager real
  queda aparte y se ejecuta con `--ignored` en una sesión de escritorio.
- CI: el workflow Build ahora corre los tests de todo el workspace Rust, no
  solo `src-tauri`, para detectar estos fallos antes de etiquetar.

## [0.3.3] - 2026-09-13 (no publicada)

Etiquetada pero su release falló en CI; su contenido salió en 0.3.4.

## [0.3.2] - 2026-09-13

### Fixed

- Antigravity nunca lograba usar la sesión de Google guardada: el código
  buscaba la credencial con `Entry::new("gemini", "antigravity")`, que en
  Windows arma el target `antigravity.gemini`, pero el propio cliente Go de
  Antigravity la guarda como `gemini:antigravity`. Además, `get_password()`
  decodifica el blob como UTF-16 (la convención de Windows para
  contraseñas), pero ese blob es JSON UTF-8 crudo, así que devolvía basura
  en vez de error. Ahora se usa el target real y se lee con `get_secret()`.
- El botón "Configurar credencial" de la tarjeta de error de un proveedor
  abría Ajustes → General en vez de la sección de ese proveedor.
- El mensaje "la credencial se escribió pero no pudo recuperarse" ahora
  incluye el error real de `get_password`/`get_secret` en vez de un texto
  genérico, para diagnosticar más rápido si vuelve a aparecer.

### Changed

- El proveedor marcado como "Primario" (Ajustes → General) ahora también
  encabeza la fila de pestañas del dashboard, no solo la preselección
  inicial.
- Quitado el encabezado duplicado (icono + nombre + plan) del panel de
  detalle: esa información ya vive en la pestaña activa, que ahora muestra
  el plan como subtítulo.
- Nuevo motor de recomendación de proveedor: sustituye la heurística de
  "más margen" por un puntaje que combina margen corto/largo, sostenibilidad
  hasta el próximo reset, ventaja de reset y confianza de datos, con
  histéresis para evitar cambios de sugerencia neuróticos.

## [0.3.1] - 2026-09-13

### Fixed

- El backend de `keyring` no tenía habilitado ningún backend nativo por
  sistema operativo, así que en Windows caía en su almacén "mock" en
  memoria: guardar una API key o la frase secreta de sync parecía funcionar
  pero la lectura inmediata siempre fallaba, y nada sobrevivía a un reinicio.
  Habilitado `windows-native`/`apple-native`/`linux-native-sync-persistent`.
- El QR de vinculación móvil podía anunciar la IP de un adaptador Ethernet
  inactivo en vez de la del Wi-Fi real cuando ambos tenían puerta de enlace
  por defecto activa. La selección de IP LAN ahora descarta interfaces
  virtuales y de loopback/APIPA y prioriza Wi-Fi sobre Ethernet sobre
  cualquier otra.
- El servidor de sync podía quedar marcado como "activo" en la interfaz
  aunque el bind del puerto hubiera fallado o el hilo hubiera muerto solo;
  `sync_set_enabled`/`sync_set_lan` ahora revierten el ajuste y devuelven el
  error real cuando el servidor no arranca de verdad.

## [0.3.0] - 2026-09-13

### Added

- Vinculación móvil completa mediante `iausage://pair`: Android abre el flujo
  desde Cámara o Lens, valida el enlace y conserva de forma segura el estado
  de la frase secreta durante recomposiciones.
- Diagnóstico y reparación del CLI desde la aplicación, junto con un smoke
  test del instalador que comprueba el binario y el PATH de usuario desde una
  PowerShell nueva.
- Nueva experiencia Android, widget configurable, recursos de marca y una
  suite ampliada de pruebas unitarias e instrumentadas.
- Extensión VS Code 0.4.0 con personalización de métricas, proveedores,
  apariencia, localización y cobertura automatizada del menú y tooltip.

### Changed

- Navegación de escritorio simplificada: el menú global dirige a categorías
  concretas y las acciones contextuales viven únicamente dentro del proveedor.
- La vinculación exige frase secreta, sync activo, LAN habilitada y una IP
  alcanzable; el QR ya no presenta `localhost` o `127.0.0.1` al teléfono.
- La interfaz visible adopta el nombre `IA Usage` y obtiene dinámicamente la
  versión de la aplicación, manteniendo los identificadores técnicos heredados
  para conservar compatibilidad de actualización y credenciales.

### Fixed

- Habilitar un proveedor actualiza en el sitio el campo de API key sin cerrar
  el detalle, perder foco ni borrar texto ante eventos del dashboard.
- Las frases secretas de sync y la eliminación de API keys usan la lectura
  tolerante a NUL de Windows Credential Manager.
- El switch de sync revierte su estado visual si el backend rechaza la
  activación, y el flujo de QR espera a que el servidor esté realmente listo.
- El instalador NSIS administra el segmento exacto de `resources\\bin` en el
  PATH de usuario, evita duplicados y lo retira con precisión al desinstalar.

## [0.2.7] - 2026-09-13

### Fixed

- Guardar una API key podía fallar con "La credencial se escribió pero
  no pudo recuperarse del almacén seguro" aunque el valor sí se hubiera
  guardado: Windows Credential Manager puede rellenar el blob con un
  byte nulo final, lo que rompe la decodificación UTF-8 estricta de
  `get_password()`. Ahora se usa `get_secret()` como respaldo, tal
  como recomienda la propia librería `keyring`.
- Pantalla de sync: guardar la passphrase reconstruía la vista con el
  estado viejo y recién después pedía el nuevo, así que el botón
  "Olvidar" y el aviso de guardado no aparecían hasta cambiar de
  pestaña. `Sincronizar con el teléfono` ahora se deshabilita hasta
  guardar una passphrase, con aviso explícito.

### Changed

- Copy de la pantalla de sync: "Passphrase" pasa a llamarse "Frase
  secreta de sincronización", con una explicación de para qué sirve y
  un aviso "Guardada de forma segura" cuando ya existe una.

## [0.2.6] - 2026-09-12

### Added

- Extensión VS Code: la barra de estado usa código de 3 letras
  (`CLD`/`CDX`/`CUR`), y el tooltip muestra el logo real de cada
  proveedor (mismo SVG de marca que ya usa la app de escritorio).

### Fixed

- Desktop: `showCommandError` saneaba el error real del backend pero
  nunca lo mostraba — toda falla de sync (sin passphrase, sync
  desactivado, LAN rechazado) se veía igual y no decía qué pasó.
  Ahora se muestra el motivo real. "Exportar ahora" se deshabilita
  hasta activar sync, en vez de fallar silenciosamente.
- Android: el widget podía quedar en "No se puede mostrar el
  contenido" de forma permanente si `provideGlance` lanzaba una
  excepción. Se blinda la lectura del snapshot y se define
  `initialLayout`/`targetCellWidth`/`targetCellHeight` en el widget
  provider.

## [0.2.5] - 2026-09-12

### Fixed

- Release Windows: `SHA256SUMS.txt` guardaba los nombres de instalador con
  espacios, pero GitHub sirve esos assets con espacios reemplazados por
  puntos; el updater nunca encontraba el checksum ("No hay checksum para
  IA.Usage.Bar_..."). Ahora el checksum se genera con el nombre que el
  asset tendrá realmente en GitHub.
- Android: la release firmada crasheaba al abrir. `isMinifyEnabled` +
  `isShrinkResources` nunca se habían probado en un build real (el CI solo
  corre `testDebugUnitTest`, una prueba JVM); se desactivan hasta poder
  diagnosticar con un logcat real qué necesita una regla `-keep`.

## [0.2.4] - 2026-09-12

### Added

- M6: companion Android con pareo QR, validación de fingerprint, descifrado
  local del envelope M5, caché protegida por Android Keystore, dashboard,
  refresco en segundo plano y widget de inicio.
- `iausage watch --jsonl`: stream persistente de snapshots para clientes
  visuales. Codex observa cambios de sesión locales con debounce de dos
  segundos y conserva el polling remoto como respaldo limitado.
- Extensión IA Usage para VS Code y forks compatibles: barra de estado,
  tooltip, refresco manual y empaquetado `.vsix` en CI/releases.

### Fixed

- El estado de credenciales se reconcilia después de guardar y validar, sin
  desincronizar los controles de Ajustes ni ocultar acciones de recuperación.
- Release Android: R8 fallaba al minificar por referencias AWT desktop-only
  de la dependencia JNA; se silencian con una regla `-dontwarn` ya que ese
  código nunca se ejecuta en Android.
- VSIX de la extensión reducido de ~1.3 MB a ~100 KB: ícono redimensionado a
  256×256 y `.gitignore`/`.github` excluidos del paquete.

## [0.2.2] - 2026-09-12

### Added

- Actualización en un clic para Windows: descarga el instalador del release,
  verifica su SHA-256 publicado y lo ejecuta sin abrir el navegador.
- Dashboard normal con Detalles, Uso por producto y Acciones plegables por
  proveedor; su estado se conserva entre refreshes y reinicios.

### Fixed

- Antigravity prueba el servidor local como Connect RPC (HTTPS, CSRF de los
  argumentos del proceso y `Connect-Protocol-Version: 1`) y usa endpoints de
  compatibilidad antes del fallback Cloud por proyecto.

## [0.2.1] - 2026-09-12

### Fixed

- Ajustes ya no reconstruye controles nativos durante un refresh: los cambios
  de proveedor y fuente son optimistas, reversibles ante error y se aplican
  por parche incremental.
- El refresco manual comunica su progreso, conserva claramente los datos
  antiguos cuando falla y separa el último intento del último dato válido.
- Codex reemplaza `auth.json` de forma atómica también en Windows; Claude y
  Codex pueden abrir sus flujos oficiales de inicio de sesión desde la UI.
- OpenAI Admin exige una Admin key con `api.usage.read`; respuestas parciales
  de proveedores ya no se presentan como 0% ni como saldo/coste $0.
- Antigravity puede continuar desde Local a Cloud y muestra la fuente que
  produjo los datos. Cursor conserva su cuota semanal de Grok Bot.
- La distribución valida el workspace Rust completo antes de publicar.

### Fixed

- Ajustes: los clics en controles sin acción propia ya no caen en la rama
  de tema vía `<html data-theme>` (reconstruía el panel a mitad del gesto
  y mataba toggles, selects y la acción pulsada). La rama de tema solo
  acepta controles reales.
- Ajustes: las filas de proveedor vuelven a plegarse (el toggle buscaba
  `[data-provider]` y las filas usan `data-provider-item`).
- Ajustes: activar, guardar credencial y detectar avisan con toast si el
  backend falla (antes quedaban en silencio) y el check revierte.
- Sync: `detect_providers` ya no mantiene locks durante el sondeo, y los
  hijos (`gh`, sondas) nunca heredan stdin (un hijo colgado no congela
  comandos ni refresh).

### Added

- M5 sync con el teléfono (V1 local, sin backend): `SyncPayload` cifrado
  (Argon2id → XChaCha20-Poly1305), blob en carpeta tras cada refresh,
  HTTP local (`/v1/meta`, `/v1/snapshot`), pareo por QR con fingerprint
  de verificación, y sección Sync en Ajustes. Ver `SYNC.md`.
- CLI `iausage sync`: `export`, `verify`, `status`, `set-passphrase`,
  `enable`/`disable`, `serve`, `qr`.

## [0.2.0] - 2026-09-12

First release under the IA Usage Bar name. `v0.1.0` was the original Claude
Bar fork this project started from; this is the reconstructed multi-provider
monitor.

### Added

- Multi-provider catalog (Claude Code, Codex/ChatGPT, Cursor, Antigravity,
  GitHub Copilot, OpenAI API, OpenRouter, Z.AI, DeepSeek, Grok, Kimi, and more)
  with a shared identity registry, local official icons, and explicit
  connection states instead of a generic connected/error boolean.
- Canonical quota, reset, credit, product-breakdown, and cost models shared
  by every provider adapter.
- Configurable notification thresholds and launch-at-login setting.
- Pin (always-on-top) and compact window modes, both persisted and mirrored
  in the tray menu.
- Header quick-actions menu (compact mode, notifications, spend, provider
  usage/status links, logs folder, settings).
- Categorized Settings screen (General, Providers, Notifications,
  Appearance, Data & logs, About).
- Minimal capped log file and a diagnostics export for bug reports.
- Windows taskbar, minimize, close-to-tray, and single-instance behavior.

- Workspace `crates/iausage-core`: todo el pipeline de providers (modelo,
  config, caché, fetchers, coste) sale de `src-tauri`, que queda como thin
  wrapper de ventana/tray. La GUI y el CLI consumen el mismo pipeline.
- `ProviderDescriptor`: fuente única de metadata (estrategias, capacidades)
  para los 23 vendors. Añadir un provider es descriptor + parser + fixtures
  + `docs/providers/<slug>.md`.
- CLI `iausage`: `usage`, `providers`, `best`, `doctor`, `refresh`, `guard`
  (exit codes 0/1/64/69), `enable`/`disable`, `config validate`, `version`,
  todo con `--json` sobre el contrato estable `DashboardSnapshotV1`
  (`schemaVersion: 1`).
- Selección de fuente por provider (`Automática/OAuth/CLI/API/Web/Local`)
  con fuente activa visible ("Usando ahora").
- Salud del servicio separada de la conexión (dos filas en Detalles).
- `DataConfidence`: los costes de logs locales se etiquetan como
  `Estimado`, nunca como factura.
- Modelo de pace formal (% real vs % esperado, proyección de agotamiento).
- Refresh adaptativo (2/5/15/30 min por actividad + ahorro de batería) con
  política pura y testeable; intervalos manuales 1/2/5/15/30.
- Comando Tauri `get_snapshot_v1` + `set_source_preference`.
- Docs por provider (`docs/providers/`) y `docs/llms.txt`.
### Changed

- Transient provider failures preserve the last valid metrics as stale data.
- API keys entered in Settings are stored in Windows Credential Manager.
- Product identity is now IA Usage Bar; see [NOTICE](NOTICE) for the
  third-party attribution this carries forward from Claude Bar and the other
  MIT-licensed projects it draws on.

- `refresh_minutes` ahora admite 1/2/5/15/30 (el antiguo 10 migra a 15).
- La caché 0.2.0 sigue siendo legible (campos nuevos con default).

[Unreleased]: https://github.com/Shadelight/ia-usage-bar/compare/v0.2.2...HEAD
[0.2.7]: https://github.com/Shadelight/ia-usage-bar/releases/tag/v0.2.7
[0.2.6]: https://github.com/Shadelight/ia-usage-bar/releases/tag/v0.2.6
[0.2.5]: https://github.com/Shadelight/ia-usage-bar/releases/tag/v0.2.5
[0.2.4]: https://github.com/Shadelight/ia-usage-bar/releases/tag/v0.2.4
[0.2.2]: https://github.com/Shadelight/ia-usage-bar/releases/tag/v0.2.2
[0.2.1]: https://github.com/Shadelight/ia-usage-bar/releases/tag/v0.2.1
[0.2.0]: https://github.com/Shadelight/ia-usage-bar/releases/tag/v0.2.0
