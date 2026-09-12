# IA Usage for VS Code

La extensión muestra cuotas de IA en la barra de estado sin implementar
proveedores ni guardar credenciales. Inicia una sola instancia de:

```text
iausage watch --jsonl
```

El CLI emite un snapshot V1 al conectar, cuando cambian sesiones locales de
Codex (debounce de dos segundos) y en el intervalo remoto de respaldo. La
extensión solo consume stdout JSON Lines.

## Desarrollo y VSIX

```powershell
npm install
npm run bundle
# Abrir esta carpeta en VS Code y pulsar F5
npm run package
```

El paquete resultante `ia-usage-0.1.0.vsix` se instala desde **Extensions →
Install from VSIX** o con `code --install-extension`. La extensión resuelve el
CLI en este orden: `iaUsage.cliPath`, `PATH` y las ubicaciones conocidas del
instalador de IA Usage Desktop. Si no lo encuentra, muestra un error con el
botón **Configurar CLI**. Para desarrollo, apunta `iaUsage.cliPath` a
`D:\\Users\\Escritorio\\IA Usage Bar\\target\\release\\iausage.exe`.

El mismo VSIX puede probarse en VS Code, Cursor, Windsurf o VSCodium si el
fork acepta extensiones compatibles. La distribución pública se realizará por
Visual Studio Marketplace, Open VSX y las GitHub Releases del proyecto.
