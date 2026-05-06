# ctx-lite Windows Installer
# Usage: powershell -Command "iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/spahmonk/ai-helpers/main/scripts/install.ps1'))"
#
# Custom install directory (no admin required):
#   $env:CTX_LITE_INSTALL_DIR = "$env:USERPROFILE\.local\bin"
#   iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/spahmonk/ai-helpers/main/scripts/install.ps1'))

$ErrorActionPreference = "Stop"
# Enforce TLS 1.2 — required by GitHub; older Windows PowerShell defaults to TLS 1.0
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

# Install directory: env override -> LocalAppData (no admin needed) -> fallback
if ($env:CTX_LITE_INSTALL_DIR) {
    $InstallDir = $env:CTX_LITE_INSTALL_DIR
} else {
    $InstallDir = "$env:LOCALAPPDATA\Programs\ctx-lite"
}

function Write-Success($Message) { Write-Host "[OK] $Message" -ForegroundColor Green }
function Write-Err($Message)     { Write-Host "[!!] $Message" -ForegroundColor Red }
function Write-Info($Message)    { Write-Host "[..] $Message" -ForegroundColor Yellow }

function Get-LatestVersion {
    $api = "https://api.github.com/repos/spahmonk/ai-helpers/releases/latest"
    # Invoke-RestMethod (PowerShell 3+) — note: -UseBasicParsing is for Invoke-WebRequest only
    try {
        $release = Invoke-RestMethod -Uri $api -ErrorAction Stop
        $tag = $release.tag_name
        if ($tag) { return ($tag -replace '^v', '') }
    } catch {}
    # Fallback: WebClient with User-Agent header (required by GitHub API)
    try {
        $wc = New-Object System.Net.WebClient
        $wc.Headers.Add("User-Agent", "ctx-lite-installer/1.0")
        $resp = $wc.DownloadString($api)
        if ($resp -match '"tag_name"\s*:\s*"v?([^"]+)"') { return $Matches[1] }
    } catch {}
    return $null
}

function Detect-Architecture {
    # Use OS bitness, not process bitness (32-bit PS can run on 64-bit OS)
    if ([Environment]::Is64BitOperatingSystem) { return "x86_64-pc-windows-msvc" }
    Write-Err "32-bit Windows is not supported"
    exit 1
}

function Main {
    Write-Host "ctx-lite Windows Installer" -ForegroundColor Cyan
    Write-Host ""

    # Auto-detect latest version
    Write-Info "Detecting latest version..."
    $Version = Get-LatestVersion
    if (-not $Version) {
        Write-Err "Could not detect latest version from GitHub API."
        Write-Err "Set CTX_LITE_VERSION env var to override, e.g.: `$env:CTX_LITE_VERSION = '1.0.7'"
        exit 1
    }
    # Allow CTX_LITE_VERSION env override; strip leading 'v' if present
    if ($env:CTX_LITE_VERSION) { $Version = $env:CTX_LITE_VERSION -replace '^v', '' }
    Write-Success "Version: $Version"

    $Platform = Detect-Architecture
    Write-Host "Platform: $Platform" -ForegroundColor Gray

    # Temp directory
    $TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
    New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

    # Download
    $DownloadUrl = "https://github.com/spahmonk/ai-helpers/releases/download/v$Version/ctx-lite-$Version-$Platform.zip"
    $ZipPath = "$TempDir\ctx-lite.zip"
    Write-Info "Downloading ctx-lite $Version..."
    Write-Host "  $DownloadUrl" -ForegroundColor Gray
    try {
        (New-Object System.Net.WebClient).DownloadFile($DownloadUrl, $ZipPath)
    } catch {
        Write-Err "Download failed: $($_.Exception.Message)"
        Write-Host "  URL: $DownloadUrl" -ForegroundColor Gray
        Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
        exit 1
    }
    Write-Success "Downloaded"

    # Extract
    Write-Info "Extracting..."
    Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force
    Write-Success "Extracted"

    # Find binary
    $BinaryPath = $null
    if (Test-Path "$TempDir\ctx-lite.exe")     { $BinaryPath = "$TempDir\ctx-lite.exe" }
    elseif (Test-Path "$TempDir\bin\ctx-lite.exe") { $BinaryPath = "$TempDir\bin\ctx-lite.exe" }
    if (-not $BinaryPath) {
        Write-Err "Binary not found in downloaded archive"
        Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
        exit 1
    }

    # Install
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }
    Write-Info "Installing to $InstallDir..."
    Copy-Item -Path $BinaryPath -Destination "$InstallDir\ctx-lite.exe" -Force
    Write-Success "Binary installed"

    # Add to user PATH (single-line call — avoids iex multi-line parsing issues)
    $CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($CurrentPath -notlike "*$InstallDir*") {
        Write-Info "Adding to user PATH..."
        [Environment]::SetEnvironmentVariable("Path", "$CurrentPath;$InstallDir", "User")
        $env:Path += ";$InstallDir"
        Write-Success "Added to PATH (open a new terminal for changes to take effect)"
    }

    # Verify
    Write-Info "Verifying..."
    $ExePath = "$InstallDir\ctx-lite.exe"
    if (Test-Path $ExePath) {
        $VersionOutput = & $ExePath --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Host ""
            Write-Success "ctx-lite installed successfully!"
            Write-Host "  Location : $ExePath"
            Write-Host "  Version  : $VersionOutput"
            Write-Host ""
            Write-Host "Try it out:" -ForegroundColor Green
            Write-Host "  ctx-lite --help" -ForegroundColor Yellow
            Write-Host "  ctx-lite tree ." -ForegroundColor Yellow
        } else {
            Write-Err "Binary verification failed"
            exit 1
        }
    } else {
        Write-Err "Installation file not found at $ExePath"
        exit 1
    }

    # Cleanup
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
}

Main
