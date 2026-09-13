# Windows CLI release smoke

The NSIS installer must place `iausage.exe` at
`$InstallDir\resources\bin\iausage.exe` and add that exact directory as one
segment of the current user's `HKCU\Environment\Path`.

Run after a clean install or update:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/smoke-installed-cli.ps1 -InstallDir "$InstallDir"
```

The smoke verifies the physical file, direct `--version`, exact per-user PATH
membership, and command discovery in a clean PowerShell process. An already
open terminal keeps the environment inherited when it started; reopen it (or
refresh `$env:Path`) before judging `Get-Command iausage` after installation.

Manual release checks from a newly opened PowerShell:

```powershell
Get-Command iausage
iausage --version
iausage usage anthropic
iausage providers
```
