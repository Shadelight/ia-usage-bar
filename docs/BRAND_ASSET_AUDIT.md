# Auditoría de identidad visual e iconografía de IA Usage

**Fecha:** 2026-09-13

**Estado:** auditoría terminada e implementación completada con la identidad oficial `IA Usage`.
**Alcance:** fuentes del repositorio, configuración de empaquetado, sitio, documentación, Android, Tauri/Windows, extensión de VS Code y workflows. Se excluyeron `node_modules/`, `target/` y `dist/` por ser salidas generadas o dependencias.

## Resumen ejecutivo

- La auditoría encontró tres identidades legacy distintas entre Desktop/Web, VS Code y Android. La implementación posterior las unificó bajo `IA Usage`.
- El repositorio ahora contiene seis maestros SVG independientes: app icon, logomark, small mark, monochrome y ambos wordmarks.
- `scripts/generate-brand-assets.mjs` deriva y distribuye de forma reproducible los formatos de Windows, Android, VS Code y web.
- Tauri cubre ejecutable, ventana, taskbar, Alt+Tab, accesos directos, MSI y NSIS desde el mismo juego de iconos. El ICO actual no contiene las capas 20x20 ni 40x40 solicitadas.
- El tray no consume un archivo: `src-tauri/src/tray.rs` dibuja un icono de 32x32 dinámicamente. Sin datos muestra las tres barras de la marca anterior; con datos muestra porcentaje, anillo de severidad y fondo oscuro.
- Android solo declara un vector legacy como `@drawable/ic_launcher`. No existen recursos por densidad, round icon, Adaptive Icon, monochrome, splash de marca ni notification small icon. El widget solo muestra el texto “IA Usage”.
- Web tiene favicon e icono de header, pero no `apple-touch-icon`, web manifest ni iconos PWA. OpenGraph/Twitter reutilizan una captura 1080x676, no una composición 1200x630.
- README usa una captura de la aplicación como imagen principal. Las capturas aún contienen branding anterior y deben regenerarse después de implementar la identidad; no deben retocarse a mano.
- Los 22 archivos físicos de proveedores (19 en Desktop y 3 en VS Code) y todos los iconos funcionales inline están fuera del reemplazo de marca.

## Requisitos resueltos para implementar

Los siguientes maestros oficiales quedaron creados como archivos separados y no como recortes de una lámina rasterizada:

| Maestro requerido | Formato preferido | Uso |
|---|---|---|
| `ia-usage-mark.svg` | SVG con `viewBox` ajustado | logomark general, 32–64 px |
| `ia-usage-small-mark.svg` | SVG simplificado | 16–32 px, favicon |
| `ia-usage-mono.svg` | SVG de un solo color, sin fondo | tray, status bar, notificaciones |
| `ia-usage-app-icon.svg` o PNG 1024x1024 | SVG o PNG lossless | Windows, Android, VS Code, touch icon |
| `ia-usage-wordmark-light.svg` | SVG | headers/README sobre fondos claros |
| `ia-usage-wordmark-dark.svg` | SVG | headers/instalador sobre fondos oscuros |

El fondo oficial del app icon es `#18181B`. El tagline quedó excluido de los lockups de producto y de todos los tamaños pequeños.

## Inventario físico encontrado

### Assets de producto

| Archivo | Formato / tamaño | Estado visual | Referencia actual |
|---|---:|---|---|
| `src-tauri/icons/32x32.png` | PNG 32x32, 523 B | tres barras verdes sobre fondo oscuro | `src-tauri/tauri.conf.json` → `bundle.icon` |
| `src-tauri/icons/128x128.png` | PNG 128x128, 2,024 B | misma identidad legacy | `src-tauri/tauri.conf.json` → `bundle.icon` |
| `src-tauri/icons/128x128@2x.png` | PNG 256x256, 4,873 B | misma identidad legacy | `src-tauri/tauri.conf.json` → `bundle.icon` |
| `src-tauri/icons/icon.ico` | ICO 16/24/32/48/64/128/256, 372,526 B | misma identidad legacy | bundle Tauri + icono instalador/desinstalador NSIS |
| `src-tauri/windows/header.bmp` | BMP 150x57, 25,818 B | icono legacy + texto “IA Usage Bar” | header de instalador y desinstalador NSIS |
| `website/icon.png` | PNG 256x256, 4,873 B | duplicado exacto del PNG Tauri 256x256 | header web a 22x22 |
| `website/favicon.ico` | ICO 16/24/32/48/64/128/256, 372,526 B | duplicado exacto del ICO Tauri | `<link rel="icon">` |
| `apps/vscode-extension/images/icon.png` | PNG 256x256, 94,670 B | cuadrícula/rombo azul-violeta, identidad distinta | `package.json.icon`, Marketplace/VSIX |
| `android/app/src/main/res/drawable/ic_launcher.xml` | Vector 48dp | tres barras horizontales verdes sobre rectángulo azul | `AndroidManifest.xml` → `android:icon` |
| `docs/design/legacy-icon.ico` | ICO 16/24/32/48/64/256, 32,251 B | archivo histórico anterior | preservado intencionalmente por planes/especificaciones históricas |

### Capturas con branding anterior

| Fuente | Duplicado exacto | Tamaño | Referencia |
|---|---|---:|---|
| `docs/screenshots/panel.png` | `website/assets/hero.png` | 1080x676 | README y hero/OpenGraph web |
| `docs/screenshots/compact.png` | `website/assets/compact.png` | 1080x383 | README y web |
| `docs/screenshots/settings.png` | `website/assets/settings.png` | 1080x548 | README y web |

Estas capturas no deben editarse automáticamente. Tras completar el cableado de marca, se deben volver a capturar desde builds reales. Las capturas de evidencia bajo `docs/superpowers/evidence/` y los documentos históricos bajo `docs/superpowers/` deben conservarse como registro, aunque enseñen estados anteriores.

### Assets que no pertenecen a IA Usage

Los siguientes deben preservarse sin cambios:

- `src/assets/providers/*.svg`: 19 logos de proveedores, cargados por `src/providers.ts`.
- `apps/vscode-extension/media/providers/{anthropic,openai,cursor}.svg`: logos de proveedores del tooltip de VS Code.
- SVG funcionales inline de `index.html`: refresh, pin, menu, minimize, close, back y add.
- SVG funcionales generados por `src/provider-actions.ts`: settings, copy, external link, warning, check, eye, refresh y demás acciones.
- Codicons de `apps/vscode-extension/src/status-bar.ts`: `pulse`, `clock`, `check`, `warning`, `refresh` y `plug`; actualmente indican estado o acción, no la marca.
- Badges de shields.io del README y badges/estados de proveedores.

## Mapa por superficie

### Windows / Tauri

| Superficie | Archivo actual | Referencia | Tamaño actual | Variante necesaria | Acción propuesta |
|---|---|---|---:|---|---|
| Ejecutable / Explorer | `src-tauri/icons/icon.ico` | `tauri.conf.json:bundle.icon` | 16–256; faltan 20 y 40 | App Icon | regenerar ICO 16/20/24/32/40/48/64/128/256 |
| Ventana / taskbar / Alt+Tab | juego `src-tauri/icons/*` | icono por defecto de bundle Tauri; no hay override de ventana | 32/128/256/ICO | App Icon | reemplazar juego y validar en build instalado y modo dev |
| Start Menu / acceso directo | `icon.ico` vía bundle | NSIS/MSI generado por Tauri | multiresolución | App Icon | validar acceso directo nuevo y actualización sobre instalación anterior |
| Add/Remove Programs | `icon.ico` vía bundle | metadata del instalador | multiresolución | App Icon | validar icono en Apps instaladas |
| Instalador NSIS | `src-tauri/icons/icon.ico` | `installerIcon` | multiresolución | App Icon | reemplazar derivado |
| Desinstalador NSIS | `src-tauri/icons/icon.ico` | `uninstallerIcon` | multiresolución | App Icon | reemplazar derivado |
| Header NSIS | `src-tauri/windows/header.bmp` | `headerImage`, `uninstallerHeaderImage` | 150x57 | Wordmark on dark | regenerar composición exacta para el canvas NSIS |
| Tray sin datos | generado en `src-tauri/src/tray.rs` | `TrayIconBuilder.icon(tray::render(None))` | 32x32 runtime | Small Mark / Monochrome | sustituir las tres barras legacy por el small mark oficial |
| Tray con datos | generado en `src-tauri/src/tray.rs` | `update_tray_from_dashboard` | 32x32 runtime | Monochrome + estado funcional | prototipar a 16/20/24; conservar legibilidad del porcentaje o definir un overlay mínimo sin deformar la marca |
| Tooltip del tray | texto en `src-tauri/src/tray.rs` | `tooltip()` | n/a | nombre oficial | mostrar “IA Usage”; preservar contexto de proveedor/porcentaje |
| Notificaciones Windows | sin asset explícito | `tauri-plugin-notification`; llamadas en `dashboard.rs`/`lib.rs` | heredado por Windows | App Icon / identidad del ejecutable | validar en build empaquetado; no añadir un icono de UI como sustituto |
| Header de la app | texto `IA Usage` | `index.html:.head-title` | texto | Wordmark o small mark discreto | usar wordmark solo si cabe en 410 px; en compact/mobile-size usar mark + texto accesible |
| Footer de la app | texto `IA Usage Bar` | `index.html:.foot-name` | texto | nombre oficial | normalizar a “IA Usage” |
| Título de ventana/producto | `IA Usage Bar` | `index.html:title`, `tauri.conf.json` | texto | nombre oficial | migrar con compatibilidad; ver riesgo de instalación abajo |

### Android

| Superficie | Archivo actual | Referencia | Tamaño actual | Variante necesaria | Acción propuesta |
|---|---|---|---:|---|---|
| Launcher | `res/drawable/ic_launcher.xml` | `AndroidManifest.xml:android:icon` | vector 48dp único | App Icon | reemplazar por recursos `mipmap-*` por densidad |
| Round launcher | no existe | `android:roundIcon` ausente | — | App Icon round-safe | crear y declarar `ic_launcher_round` |
| Adaptive foreground | no existe | `mipmap-anydpi-v26` ausente | — | Logomark | crear foreground con safe zone oficial |
| Adaptive background | no existe | `mipmap-anydpi-v26` ausente | — | color oficial | crear color/background independiente |
| Android monochrome | no existe | adaptive icon ausente | — | Monochrome | añadir `<monochrome>` para launchers compatibles |
| Splash | no existe branding específico | `Theme.IAUsage` hereda `Theme.Material.Light.NoActionBar` | — | Small Mark/App Icon | definir splash Android 12+ y fallback, con variantes light/dark |
| Notification small icon | no existe y no hay notificaciones Android implementadas | — | — | Monochrome | crear solo cuando exista una notificación; debe ser silueta blanca con transparencia, nunca PNG color |
| Widget header | solo texto `IA Usage` | `UsageWidget.kt` | texto | Small Mark / Monochrome | añadir marca pequeña junto al título; no repetirla en filas de providers |
| Widget preview/selector | no existe preview de marca | `usage_widget_info.xml` | — | captura/widget preview | evaluar `previewImage`/`previewLayout` después del diseño final |
| Pantalla de pairing | texto `IA Usage Bar` | `PairingScreen.kt` | texto | nombre oficial | normalizar a “IA Usage”; un mark pequeño es opcional |

El árbol `res/` no contiene `mipmap-mdpi`, `mipmap-hdpi`, `mipmap-xhdpi`, `mipmap-xxhdpi`, `mipmap-xxxhdpi`, `drawable-night` ni `mipmap-anydpi-v26`.

### VS Code

| Superficie | Archivo actual | Referencia | Tamaño actual | Variante necesaria | Acción propuesta |
|---|---|---|---:|---|---|
| Marketplace / extensión instalada | `apps/vscode-extension/images/icon.png` | `package.json.icon` | 256x256, 94,670 B | App Icon | reemplazar por PNG oficial optimizado, con margen óptico validado a 64/128/256 |
| Gallery banner | solo color `#1e1e1e` | `package.json.galleryBanner` | n/a | App Icon compatible con dark | validar contraste del nuevo icono contra el banner |
| Status bar | Codicons funcionales | `src/status-bar.ts` | fuente de iconos VS Code | conservar UI; opcional Product Icon mono | no reemplazar `check/clock/pulse/warning`; un mark de IA Usage solo puede identificar producto sin ocultar estado |
| Tooltip header | texto “IA Usage” | `src/status-bar.ts` | texto | nombre oficial | conservar; se puede añadir small mark mono si VS Code lo renderiza con contraste fiable |
| Tooltip providers | SVG Anthropic/OpenAI/Cursor | `media/providers` | SVG | Provider icons | preservar |
| README/CHANGELOG Marketplace | sin logo de producto incrustado | archivos Markdown | — | App Icon/wordmark opcional | usar wordmark solo si aporta contexto; no duplicar el icono Marketplace innecesariamente |

### Web / landing

| Superficie | Archivo actual | Referencia | Tamaño actual | Variante necesaria | Acción propuesta |
|---|---|---|---:|---|---|
| Header desktop | `website/icon.png` + texto | `website/index.html` | asset 256, render 22x22 | Wordmark horizontal | usar wordmark on light/on dark según tema |
| Header estrecho | mismo icono + texto | CSS responsive | 22x22 | Small Mark / Logomark | ocultar wordmark si no cabe, manteniendo nombre accesible |
| Favicon | `website/favicon.ico` | `<link rel="icon">` | ICO 16–256, 372 KB | Small Mark | generar favicon 16/32/48 y SVG cuando sea compatible |
| Apple touch icon | no existe | link ausente | — | App Icon | generar 180x180 y añadir metadata |
| Manifest/PWA | no existe | manifest ausente | — | App Icon | decidir si el sitio es instalable; si sí, añadir 192/512 y maskable |
| OpenGraph | `website/assets/hero.png` | `og:image`, `twitter:image` | 1080x676 | composición de marca | crear imagen 1200x630; no usar una captura como asset social definitivo |
| Theme color | `#1c1c1e` | meta `theme-color` | n/a | token de marca | alinear con paleta oficial, manteniendo contraste |
| Screenshots | `website/assets/*.png` | secciones de producto | 1080 px ancho | capturas reales | regenerar después de la implementación, no retocar |

### GitHub / documentación

| Superficie | Archivo actual | Referencia | Tamaño actual | Variante necesaria | Acción propuesta |
|---|---|---|---:|---|---|
| Encabezado README | `docs/screenshots/panel.png` | `README.md:3` | 1080x676 | Wordmark horizontal | sustituir la captura usada como “logo” por wordmark limpio y de ancho moderado |
| Galería README | `docs/screenshots/{panel,compact,settings}.png` | `README.md` | 1080 px ancho | capturas reales | conservar temporalmente; regenerar cuando la UI final muestre la nueva marca |
| Docs actuales | nombre “IA Usage Bar” en documentos activos | README, SECURITY, CONTRIBUTING, `docs/llms.txt`, Android README | texto | nombre oficial | normalizar documentos vigentes a “IA Usage” |
| Docs históricos | branding anterior | CHANGELOG, `docs/superpowers/**`, `docs/design/legacy-icon.ico` | histórico | archivo histórico | no reescribir; conservar trazabilidad |
| Issue templates | nombre “IA Usage Bar” | `.github/ISSUE_TEMPLATE/*.yml` | texto | nombre oficial | normalizar texto visible; no alterar campos/IDs |

### Releases y empaquetado

| Superficie | Estado actual | Acción propuesta |
|---|---|---|
| Windows EXE/MSI | `.github/workflows/release.yml` usa Tauri y publica ambos | los derivados en `src-tauri/icons/` y `windows/header.bmp` quedan incluidos por configuración; validar artefactos instalados |
| Android APK/AAB | Gradle empaqueta `res/` y workflow publica APK/AAB firmados | los nuevos `mipmap`/`drawable` quedan incluidos automáticamente; inspeccionar ambos artefactos |
| VSIX | workflow ejecuta packaging y publica `*.vsix` | comprobar con `vsce ls` que `images/icon.png` y solo los assets necesarios entren al paquete |
| Web | `.github/workflows/pages.yml` despliega el sitio estático | incluir derivados web dentro de `website/` o ajustar el workflow a una carpeta generada estable |
| Fuentes de marca | no existe política | no incluir PSD/AI/Figma exports gigantes en instaladores; versionar maestros SVG/PNG y derivados técnicos necesarios |

## Nombres: qué cambiar y qué preservar

La identidad solicitada usa siempre **IA Usage** en texto visible. Esto afecta, como mínimo:

- Título/footer del frontend Desktop: `index.html`.
- `productName`, título de ventana y nombres visibles del bundle: `src-tauri/tauri.conf.json`.
- Tooltip, menú y notificaciones: `src-tauri/src/lib.rs`, `src-tauri/src/tray.rs` y textos relacionados.
- Textos NSIS: `src-tauri/windows/English.nsh` y el raster `header.bmp`.
- Landing y metadatos sociales: `website/index.html`, `website/app.js`.
- README, documentación vigente, issue templates y nombre de release en `.github/workflows/release.yml`.
- Widget/pairing Android: `strings.xml`, `UsageWidget.kt`, `PairingScreen.kt`.
- Descripciones visibles de crates si se desea consistencia en metadata.

No se deben cambiar:

- Tauri identifier `com.alberth.iausagebar`.
- Android `applicationId`/namespace `com.shadelight.iausage`.
- VS Code extension ID derivado de `publisher` + `name`; `publisher` y `name` permanecen iguales.
- Nombres de paquetes/crates (`iausagebar`, `iausagebar_lib`) ni binario/CLI `iausage`.
- URLs del repositorio `ia-usage-bar`, protocolos, servicio de credenciales o claves de configuración.
- Rutas legacy que sean necesarias para descubrir instalaciones existentes.

Cambiar `productName` puede cambiar nombres de artefacto, carpeta instalada, acceso directo y registro del desinstalador. `apps/vscode-extension/src/cli-client.ts` busca explícitamente carpetas llamadas `IA Usage Bar`. La implementación debe buscar primero la ruta nueva y mantener la antigua como fallback durante la migración; también debe probar una actualización sobre una instalación existente, no solo una instalación limpia.

## Estructura objetivo

```text
assets/
  brand/
    source/
      ia-usage-mark.svg
      ia-usage-small-mark.svg
      ia-usage-mono.svg
      ia-usage-app-icon.svg
      ia-usage-wordmark-light.svg
      ia-usage-wordmark-dark.svg
    generated/
      windows/
        32x32.png
        128x128.png
        128x128@2x.png
        icon.ico
        nsis-header.bmp
      android/
        ...
      vscode/
        icon.png
      web/
        favicon.svg
        favicon-16.png
        favicon-32.png
        favicon-48.png
        apple-touch-icon.png
        icon-192.png
        icon-512.png
        og-image.png
```

Los destinos que las herramientas exigen (`src-tauri/icons/`, `android/app/src/main/res/`, `apps/vscode-extension/images/`, `website/`) pueden generarse/copiarse desde esta fuente única. El script debe fallar si falta un maestro, preservar transparencia/perfil sRGB y producir resultados deterministas.

## Legacy assets safe to remove

No hay ningún asset activo que sea seguro borrar en esta primera pasada. La lista queda condicionada a completar el recableado y verificar paquetes:

| Candidato | Condición para retirarlo |
|---|---|
| `src-tauri/icons/{32x32.png,128x128.png,128x128@2x.png,icon.ico}` actuales | sustituir por derivados nuevos en la misma ruta o actualizar todas las referencias Tauri; validar EXE/MSI/NSIS |
| `src-tauri/windows/header.bmp` actual | sustituir por header oficial nuevo y validar instalador/desinstalador |
| `website/icon.png`, `website/favicon.ico` actuales | actualizar HTML y verificar favicon/header en light/dark y hard refresh |
| `apps/vscode-extension/images/icon.png` actual | sustituir y confirmar contenido del VSIX/Marketplace |
| `android/app/src/main/res/drawable/ic_launcher.xml` | manifest apuntando a nuevos recursos y APK/AAB inspeccionados |
| capturas `docs/screenshots/*` y `website/assets/*` | reemplazar por nuevas capturas; no borrar mientras README/web sigan referenciándolas |

`docs/design/legacy-icon.ico` **no** está marcado como eliminable: fue conservado deliberadamente como referencia histórica. Los SVG de providers y los iconos funcionales tampoco son legacy de IA Usage.

## Riesgos y decisiones pendientes

1. **Faltan masters oficiales.** Extraer el símbolo desde la lámina adjunta obligaría a redibujar o adivinar; la implementación debe esperar archivos fuente separados.
2. **Migración del nombre de producto.** Puede afectar upgrades, rutas, shortcuts, detección desde VS Code y nombres de release. Debe preservarse compatibilidad con “IA Usage Bar”.
3. **Tray versus información.** El tray actual comunica el porcentaje primario. Sustituirlo por un logo estático elimina información; mezclar logo y porcentaje puede destruir legibilidad a 16 px. Se requiere una decisión explícita después de comparar prototipos a tamaño real.
4. **Gradientes a tamaños pequeños.** El nuevo mark color luce bien en grande, pero favicon/tray/status/notification deben usar small mark o mono, nunca el lockup completo.
5. **Adaptive Icon.** Un recorte correcto exige foreground dentro de safe zone y background separado; no basta escalar el app icon cuadrado.
6. **Notificaciones Android.** La small icon debe ser una silueta monocroma con transparencia. No debe reutilizarse el app icon.
7. **Fondos del sistema.** ICO, tray, VS Code y Android deben probarse sobre blanco, `#18181B`, temas de Windows/VS Code y launchers light/dark.
8. **Capturas duplicadas.** `website/assets/*` y `docs/screenshots/*` son copias exactas pero ambos grupos están referenciados. Deduplicar sin revisar Pages rompería rutas.
9. **Reproducibilidad.** No existe generador de assets y ImageMagick no está disponible actualmente en el entorno. La herramienta de rasterizado debe fijarse en dependencias o documentarse en CI.
10. **Validación de Windows real.** El icono visto en desarrollo puede venir de caché o del ejecutable host. Hay que validar una instalación limpia y una actualización, además de limpiar caché de iconos cuando corresponda.

## Plan de implementación propuesto

1. Recibir y validar los seis maestros oficiales: geometría, transparencia, `viewBox`, colores, variantes on-light/on-dark y safe area.
2. Añadir `assets/brand/source/` y un generador reproducible `scripts/generate-brand-assets.*`; producir únicamente derivados técnicos necesarios.
3. Generar una hoja de control a 16/20/24/32/48/64/128/256 px sobre fondos claros y oscuros. Rechazar cualquier variante que pierda la forma “IA”.
4. Recablear Windows/Tauri: bundle, window/executable, NSIS/MSI, header y comportamiento del tray. Preservar el porcentaje o acordar explícitamente su reemplazo.
5. Crear el set Android completo: densidades, round, adaptive foreground/background, monochrome y splash. Añadir marca discreta al widget; no tocar provider rows.
6. Reemplazar el icono Marketplace y validar status bar/tooltip sin convertir iconos de estado en branding.
7. Actualizar web: wordmark/header responsive, favicon, touch icon, manifest si aplica y OpenGraph 1200x630.
8. Actualizar README y documentación vigente; conservar históricos. Regenerar screenshots desde builds reales después de terminar la UI.
9. Normalizar texto visible a “IA Usage” con compatibilidad de upgrade/rutas y sin cambiar identificadores técnicos.
10. Inspeccionar artefactos finales y ejecutar la matriz de validación.

## Validación prevista

### Automatizada

```text
npm test
npm run build
cargo fmt --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri build

cd android
gradle :app:testDebugUnitTest :app:assembleDebug

cd apps/vscode-extension
npm run check
npm run bundle
npm run package
npx vsce ls

git diff --check
```

El sitio es estático y no tiene build propio; debe validarse el contenido que publica `.github/workflows/pages.yml`, además de links y metadata.

### Visual y de empaquetado

- Raster sheet a 16/20/24/32/48/64/128/256 sobre fondos blanco, oscuro y del sistema.
- Windows: Explorer, ventana, taskbar, Alt+Tab, Start Menu, acceso directo, Apps instaladas, tray idle/activo, notificación, NSIS y MSI.
- Android: launcher normal/round, recortes de varios launchers, themed icon, splash light/dark, widget selector y widget real.
- VS Code: Marketplace/Extensions view, tema claro/oscuro/alto contraste, status bar en estados waiting/fresh/stale/error y tooltip de providers.
- Web: favicon real a 16/32, Safari/touch icon, header responsive light/dark y preview social 1200x630.
- README: wordmark de tamaño moderado y nuevas capturas sin branding accidental anterior.

## Matriz de finalización esperada

| Área | Criterios |
|---|---|
| Web | favicon, header responsive, touch/PWA si aplica, OpenGraph 1200x630 |
| Windows | executable, window, taskbar, Alt+Tab, tray, installer, uninstaller, shortcuts, Apps instaladas |
| Android | launcher, round, adaptive, monochrome, splash, notification cuando exista, widget |
| VS Code | Marketplace, extension details, tooltip/provider separation, status bar funcional |
| GitHub | README, docs vigentes, screenshots regeneradas, históricos preservados |
| Releases | EXE, MSI, APK, AAB y VSIX inspeccionados; sin masters innecesarios dentro de paquetes |

La implementación usa seis maestros oficiales, un generador reproducible y un small mark monocromático adaptado al tray dinámico. La decisión visual y sus pruebas están documentadas en `docs/BRAND_IDENTITY.md`.
