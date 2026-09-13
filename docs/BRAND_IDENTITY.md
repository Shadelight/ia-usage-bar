# Identidad visual de IA Usage

## Decisión final

La identidad oficial es **IA Usage**. La `I` funciona como primera barra de cuota; la `A` contiene el ritmo ascendente de uso. El símbolo se resuelve con masas sólidas, no con efectos, y mantiene la lectura `IA` al reducirse.

Paleta oficial: fondo `#18181B`, superficie `#202023`, coral `#FF7043`, texto `#F5F5F5`, muted `#71717A` y verde de datos `#34D399`.

La propuesta azul/cian/violeta quedó descartada por completo. No se conserva como variante ni como fallback.

## Direcciones exploradas

| Dirección | Idea | Evaluación |
|---|---|---|
| **A. IA integrada** | La `I` inicia la gráfica y las barras viven dentro de la `A`. | Seleccionada: lectura más clara, mejor silueta a 16 px y mayor coherencia con la UI. |
| **B. Usage Mark** | Las barras ascendentes construyen parte de la `A`. | Potente como métrica, pero más compleja y menos literal a tamaño mínimo. |
| **C. IA Cutout** | Bloque coral con `IA` recortada por espacio negativo. | Muy reconocible a gran tamaño, demasiado pesado para tray y status bar. |

## Sistema oficial

| Superficie | Asset |
|---|---|
| Windows, Android launcher, VS Code Marketplace | `ia-usage-app-icon` |
| Header web y documentación | wordmark según fondo |
| Header compacto y widget | `ia-usage-small-mark` |
| Tray sin datos | `ia-usage-mono` simplificado |
| Tray con datos | porcentaje central + anillo de severidad |
| VS Code status bar, Android notification | `ia-usage-mono` |
| Favicon 48 px | app icon |
| Favicon 32 px | small mark sobre fondo oscuro |
| Favicon 16 px | small mark monocromático sobre fondo oscuro |

El nombre visible es siempre **IA Usage**. La tagline no forma parte de los iconos ni de la UI de producto.

Android Adaptive Icon mantiene background y foreground como capas separadas. El foreground y su variante monocromática usan un inset del 18%, sin un segundo contenedor dibujado dentro del icono.

## Evidencia visual

- [Tres direcciones](../assets/brand/generated/brand-directions.png)
- [Aplicaciones por plataforma](../assets/brand/generated/brand-applications.png)
- [Legibilidad 16–256 px](../assets/brand/generated/brand-legibility-sheet.png)

## Regeneración

Ejecutar `npm run brand:generate` desde la raíz. El comando deriva ICO/PNG/XML/BMP y distribuye los resultados a Windows/Tauri, Android, VS Code, web y la UI de escritorio.

## Validación ejecutada

- 59 pruebas frontend aprobadas.
- Build Vite de producción aprobado.
- `cargo check` y 24 pruebas Rust aprobadas.
- Bundle de producción y VSIX 0.3.2 de la extensión aprobados. El `tsc` global está bloqueado por helpers sin declarar en los tests nuevos de quick-menu/quick-settings; no afecta a los assets de marca.
- Recursos XML de Android parseados correctamente (19 archivos). El checkout no incluye Gradle wrapper y el host no expone `gradle`, por lo que no se ejecutó el APK.
- Ejecutable Tauri y bundle NSIS de Windows generados correctamente.
- El bundle MSI llega a WiX pero `light.exe` falla en este entorno; no es un error de los assets ni del código compilado.
- `git diff --check` aprobado.
