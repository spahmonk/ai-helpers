# ctx-lite Windows Installer
# Usage: powershell -Command "iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/spahmonk/ai-helpers/main/scripts/install.ps1'))"

param(
    [string]$Version = "1.0.0",
    [string]$InstallDir = "$env:ProgramFiles\ctx-lite"
)

$ErrorActionPreference = "Stop"

function Write-Success {
    param([string]$Message)
    Write-Host "✓ $Message" -ForegroundColor Green
}

function Write-Error_ {
    param([string]$Message)
    Write-Host "✗ $Message" -ForegroundColor Red
}

function Write-Info {
    param([string]$Message)
    Write-Host "⚙️  $Message" -ForegroundColor Yellow
}

function Detect-Architecture {
    $arch = [Environment]::Is64BitProcess
    if ($arch) {
        return "x86_64-pc-windows-msvc"
    } else {
        Write-Error_ "32-bit Windows is not supported"
        exit 1
    }
}

function Main {
    Write-Info "ctx-lite installer v$Version"
    Write-Host ""
    
    # Check prerequisites
    Write-Info "Checking prerequisites..."
    if (-not (Get-Command curl -ErrorAction SilentlyContinue)) {
        Write-Error_ "curl is not available. Please install curl or use another method."
        exit 1
    }
    
    # Detect platform
    $Platform = Detect-Architecture
    Write-Host "Detected platform: " -NoNewline
    Write-Host "$Platform" -ForegroundColor Green
    
    # Create temp directory
    $TempDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }
    Write-Host "Temp directory: $TempDir" -ForegroundColor Gray
    
    # Download
    $DownloadUrl = "https://github.com/spahmonk/ai-helpers/releases/download/v$Version/ctx-lite-$Version-$Platform.zip"
    $ZipPath = "$TempDir\ctx-lite.zip"
    
    Write-Info "Downloading ctx-lite $Version..."
    Write-Host "URL: $DownloadUrl" -ForegroundColor Gray
    
    try {
        (New-Object System.Net.WebClient).DownloadFile($DownloadUrl, $ZipPath)
    } catch {
        Write-Error_ "Failed to download from: $DownloadUrl"
        Write-Error_ $_.Exception.Message
        Write-Host "Make sure version $Version is released on GitHub." -ForegroundColor Gray
        exit 1
    }
    
    Write-Success "Downloaded"
    
    # Extract
    Write-Info "Extracting..."
    Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force
    Write-Success "Extracted"
    
    # Find binary
    $BinaryPath = $null
    if (Test-Path "$TempDir\ctx-lite.exe") {
        $BinaryPath = "$TempDir\ctx-lite.exe"
    } elseif (Test-Path "$TempDir\bin\ctx-lite.exe") {
        $BinaryPath = "$TempDir\bin\ctx-lite.exe"
    }
    
    if (-not $BinaryPath) {
        Write-Error_ "Binary not found in downloaded archive"
        exit 1
    }
    
    # Create install directory if needed
    if (-not (Test-Path $InstallDir)) {
        Write-Info "Creating installation directory: $InstallDir"
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }
    
    # Copy binary
    Write-Info "Installing to $InstallDir..."
    Copy-Item -Path $BinaryPath -Destination "$InstallDir\ctx-lite.exe" -Force
    Write-Success "Installed"
    
    # Add to PATH (current session)
    $CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($CurrentPath -notlike "*$InstallDir*") {
        Write-Info "Adding to user PATH..."
        [Environment]::SetEnvironmentVariable(
            "Path",
            "$CurrentPath;$InstallDir",
            "User"
        )
        Write-Success "Added to PATH (restart terminal for changes to take effect)"
        # Also add to current session
        $env:Path += ";$InstallDir"
    }
    
    # Verify installation
    Write-Info "Verifying installation..."
    $ExePath = "$InstallDir\ctx-lite.exe"
    if (Test-Path $ExePath) {
        $VersionOutput = & $ExePath --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Host ""
            Write-Success "Successfully installed!"
            Write-Host "  Location: $ExePath"
            Write-Host "  Version: $VersionOutput"
            Write-Host ""
            Write-Host "You're all set! Try:" -ForegroundColor Green
            Write-Host "  ctx-lite --help" -ForegroundColor Yellow
        } else {
            Write-Error_ "Binary verification failed"
            exit 1
        }
    } else {
        Write-Error_ "Installation file not found at $ExePath"
        exit 1
    }
    
    # Cleanup
    Remove-Item -Recurse -Force $TempDir
}

Main
