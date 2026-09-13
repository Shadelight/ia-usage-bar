param(
  [Parameter(Mandatory = $true)]
  [string]$InstallDir
)

$ErrorActionPreference = 'Stop'
$cliDir = Join-Path $InstallDir 'resources\bin'
$cli = Join-Path $cliDir 'iausage.exe'

if (-not (Test-Path -LiteralPath $cli -PathType Leaf)) {
  throw "Bundled CLI is missing: $cli"
}

$directVersion = & $cli --version
if ($LASTEXITCODE -ne 0 -or -not $directVersion) {
  throw "Direct CLI smoke failed: $cli --version"
}

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($null -eq $userPath) { $userPath = '' }
$segments = $userPath.Split(';', [StringSplitOptions]::RemoveEmptyEntries) |
  ForEach-Object { $_.Trim().Trim('"').TrimEnd('\', '/') }
$expected = $cliDir.TrimEnd('\', '/')
if (-not ($segments | Where-Object { $_.Equals($expected, [StringComparison]::OrdinalIgnoreCase) })) {
  throw "User PATH does not contain the exact CLI directory: $cliDir"
}

# A genuinely new terminal gets the environment Windows rebuilt after
# WM_SETTINGCHANGE. CI children inherit the runner's older environment, so
# refresh this parent from the machine + user registry values before spawning
# the clean PowerShell process that exercises normal command discovery.
$machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
if ($null -eq $machinePath) { $machinePath = '' }
$env:Path = "$machinePath;$userPath"
$probe = Start-Process powershell.exe -NoNewWindow -Wait -PassThru -ArgumentList @(
  '-NoProfile',
  '-Command',
  'Get-Command iausage -ErrorAction Stop; iausage --version'
)
if ($probe.ExitCode -ne 0) {
  throw "Fresh PowerShell CLI discovery failed with exit code $($probe.ExitCode)"
}

Write-Host "CLI smoke passed: $directVersion"
