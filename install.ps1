# Install the standalone nio-js executable from an official GitHub release.
[CmdletBinding()]
param(
  [Parameter(Position = 0)]
  [string]$Version = $env:NIO_JS_VERSION,
  [Parameter(Position = 1)]
  [string]$InstallDir = $env:NIO_JS_INSTALL_DIR,
  [switch]$Help
)

$ErrorActionPreference = 'Stop'

function Show-Help {
  @"
Usage: install.ps1 [-Version v0.1.0] [-InstallDir DIR]

Defaults:
  - Version: latest stable release
  - InstallDir: %USERPROFILE%\.local\bin
Environment:
  NIO_JS_VERSION, NIO_JS_INSTALL_DIR
"@
}

if ($Help) {
  Show-Help
  exit 0
}

function Fail([string]$Message) {
  Write-Error "nio-js: $Message"
  exit 1
}

# Verify architecture
$is64Bit = [System.Environment]::Is64BitOperatingSystem
if (-not $is64Bit) {
  Fail "Unsupported architecture. Windows x64 is currently required."
}

if ([string]::IsNullOrWhiteSpace($Version)) {
  $Version = "latest"
}

if ($Version -ne "latest") {
  if (-not ($Version.StartsWith("v"))) {
    $Version = "v$Version"
  }
  if ($Version -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9]+([.-][A-Za-z0-9]+)*)?$') {
    Fail "Expected a version such as v0.1.0 or v0.2.0-rc.1"
  }
}

if ([string]::IsNullOrWhiteSpace($InstallDir)) {
  $userProfile = $env:USERPROFILE
  if ([string]::IsNullOrWhiteSpace($userProfile)) {
    $userProfile = [System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::UserProfile)
  }
  if ([string]::IsNullOrWhiteSpace($userProfile)) {
    Fail "Cannot determine user profile directory. Set -InstallDir."
  }
  $InstallDir = Join-Path $userProfile ".local\bin"
}

$asset = "nio-js-win32-x64.exe"
$baseUrl = if ($Version -eq "latest") {
  "https://github.com/nio-labs/nio-js/releases/latest/download"
} else {
  "https://github.com/nio-labs/nio-js/releases/download/$Version"
}

Write-Host "Downloading nio-js (win32-x64, $Version)..."

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("nio-js-install-" + [System.Guid]::NewGuid().ToString("n"))
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

try {
  $downloadBin = Join-Path $tempDir $asset
  $downloadSha = Join-Path $tempDir "$asset.sha256"

  [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.SecurityProtocolType]::Tls12 -bor [System.Net.SecurityProtocolType]::Tls13

  try {
    Invoke-WebRequest -Uri "$baseUrl/$asset" -OutFile $downloadBin -UseBasicParsing
  } catch {
    Fail "Could not download the binary. Check that this release has been published."
  }

  try {
    Invoke-WebRequest -Uri "$baseUrl/$asset.sha256" -OutFile $downloadSha -UseBasicParsing
  } catch {
    Fail "Could not download the release checksum."
  }

  $shaContent = Get-Content -Path $downloadSha -Raw
  $expectedHash = ($shaContent.Trim() -split '\s+')[0].Trim().ToLower()
  if ($expectedHash -notmatch '^[0-9a-f]{64}$') {
    Fail "Invalid release checksum"
  }

  $actualHash = (Get-FileHash -Path $downloadBin -Algorithm SHA256).Hash.ToLower()
  if ($actualHash -ne $expectedHash) {
    Fail "Checksum mismatch; installation stopped"
  }

  try {
    $null = & $downloadBin --version 2>&1
  } catch {
    Fail "This binary cannot run on your system; the existing installation was preserved"
  }

  if (-not (Test-Path -Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
  }

  $targetExe = Join-Path $InstallDir "nio-js.exe"
  Copy-Item -Path $downloadBin -Destination $targetExe -Force

  Write-Host "Installed $targetExe"

  $envPath = [System.Environment]::GetEnvironmentVariable('Path', 'User')
  $pathParts = if ($envPath) { $envPath.Split(';') } else { @() }
  $resolvedInstallDir = (Resolve-Path $InstallDir).Path
  $inPath = $false
  foreach ($part in $pathParts) {
    if ($part.Trim() -ne "" -and (Test-Path -Path $part)) {
      if ((Resolve-Path $part).Path.Equals($resolvedInstallDir, [System.StringComparison]::OrdinalIgnoreCase)) {
        $inPath = $true
        break
      }
    }
  }

  if (-not $inPath) {
    try {
      $newPath = if ([string]::IsNullOrWhiteSpace($envPath)) { $InstallDir } else { "$envPath;$InstallDir" }
      [System.Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
      $env:PATH = "$env:PATH;$InstallDir"
      Write-Host "Added $InstallDir to user PATH."
    } catch {
      Write-Host "Add this directory to your PATH in your shell profile:`n  `$env:PATH = `"$InstallDir;`$env:PATH`""
    }
  }
} finally {
  if (Test-Path -Path $tempDir) {
    Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
  }
}
