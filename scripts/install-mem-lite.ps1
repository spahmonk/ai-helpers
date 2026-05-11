# mem-lite Windows Installer
# Usage: powershell -Command "iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/spahmonk/ai-helpers/main/scripts/install-mem-lite.ps1'))"
#
# Custom install directory (no admin required):
#   $env:MEM_LITE_INSTALL_DIR = "$env:USERPROFILE\.local\bin"
#   iex ((New-Object System.Net.WebClient).DownloadString('...'))

$ErrorActionPreference = "Stop"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

if ($env:MEM_LITE_INSTALL_DIR) {
    $InstallDir = $env:MEM_LITE_INSTALL_DIR
} else {
    $InstallDir = "$env:LOCALAPPDATA\Programs\mem-lite"
}

function Write-Success($Message) { Write-Host "[OK] $Message" -ForegroundColor Green }
function Write-Err($Message)     { Write-Host "[!!] $Message" -ForegroundColor Red }
function Write-Info($Message)    { Write-Host "[..] $Message" -ForegroundColor Yellow }

function Get-LatestVersion {
    $api = "https://api.github.com/repos/spahmonk/ai-helpers/releases"
    try {
        $releases = Invoke-RestMethod -Uri $api -ErrorAction Stop
        foreach ($r in $releases) {
            if ($r.tag_name -match '^mem-lite-v(.+)$') { return $Matches[1] }
        }
    } catch {}
    try {
        $wc = New-Object System.Net.WebClient
        $wc.Headers.Add("User-Agent", "mem-lite-installer/1.0")
        $resp = $wc.DownloadString($api)
        if ($resp -match '"tag_name"\s*:\s*"mem-lite-v([^"]+)"') { return $Matches[1] }
    } catch {}
    return $null
}

function Detect-Architecture {
    if ([Environment]::Is64BitOperatingSystem) { return "x86_64-pc-windows-msvc" }
    Write-Err "32-bit Windows is not supported"
    exit 1
}

function Main {
    Write-Host "mem-lite Windows Installer" -ForegroundColor Cyan
    Write-Host ""

    Write-Info "Detecting latest version..."
    $Version = $env:MEM_LITE_VERSION
    if ($Version) {
        $Version = $Version -replace '^mem-lite-v', ''
        $Version = $Version -replace '^v', ''
    } else {
        $Version = Get-LatestVersion
    }
    if (-not $Version) {
        Write-Err "Could not detect latest mem-lite version from GitHub API."
        Write-Err "Set MEM_LITE_VERSION env var to override, e.g.: `$env:MEM_LITE_VERSION = '0.1.0'"
        exit 1
    }
    Write-Success "Version: $Version"

    Write-Info "Detecting platform..."
    $Platform = Detect-Architecture
    Write-Success "Platform: $Platform"

    $TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
    New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

    $DownloadUrl = "https://github.com/spahmonk/ai-helpers/releases/download/mem-lite-v$Version/mem-lite-$Version-$Platform.zip"
    $ZipPath = "$TempDir\mem-lite.zip"
    Write-Info "Downloading mem-lite $Version..."
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

    Write-Info "Extracting..."
    Expand-Archive -LiteralPath $ZipPath -DestinationPath $TempDir -Force
    Write-Success "Extracted"

    $BinaryPath = $null
    if (Test-Path "$TempDir\mem-lite.exe")         { $BinaryPath = "$TempDir\mem-lite.exe" }
    elseif (Test-Path "$TempDir\bin\mem-lite.exe") { $BinaryPath = "$TempDir\bin\mem-lite.exe" }
    if (-not $BinaryPath) {
        Write-Err "Binary not found in downloaded archive"
        Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
        exit 1
    }

    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }
    Write-Info "Installing to $InstallDir..."
    Copy-Item -Path $BinaryPath -Destination "$InstallDir\mem-lite.exe" -Force
    Write-Success "Binary installed"

    $CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($CurrentPath -notlike "*$InstallDir*") {
        Write-Info "Adding to user PATH..."
        [Environment]::SetEnvironmentVariable("Path", "$CurrentPath;$InstallDir", "User")
        $env:Path += ";$InstallDir"
        Write-Success "Added to PATH (open a new terminal for changes to take effect)"
    }

    Write-Info "Verifying..."
    $ExePath = "$InstallDir\mem-lite.exe"
    if (Test-Path $ExePath) {
        $VersionOutput = & $ExePath --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Host ""
            Write-Success "mem-lite installed successfully!"
            Write-Host "  Location : $ExePath"
            Write-Host "  Version  : $VersionOutput"
            Write-Host ""
            Write-Host "Try it out:" -ForegroundColor Green
            Write-Host "  mem-lite --help" -ForegroundColor Yellow
            Write-Host "  mem-lite init" -ForegroundColor Yellow
            Write-Host "  mem-lite --mcp   # start MCP server" -ForegroundColor Yellow
        } else {
            Write-Err "Binary verification failed"
            exit 1
        }
    } else {
        Write-Err "Installation file not found at $ExePath"
        exit 1
    }

    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
}

Main
