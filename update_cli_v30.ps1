$ErrorActionPreference = "Stop"

Write-Host "Stopping existing CLI processes..."
Stop-Process -Name space-thumbnails-cli -Force -ErrorAction SilentlyContinue

$releaseDir = "$PWD\target_temp_debug_v29\release"

if (!(Test-Path $releaseDir)) {
    Write-Error "Release directory not found: $releaseDir"
    exit 1
}

Write-Host "Building CLI (release)..."
cargo build --release --bin space-thumbnails-cli
if ($LASTEXITCODE -ne 0) { Write-Error "CLI build failed"; exit 1 }

Write-Host "Deploying CLI to $releaseDir..."
Copy-Item "target\release\space-thumbnails-cli.exe" -Destination "$releaseDir\space-thumbnails-cli.exe" -Force

Write-Host "CLI Updated (v30 - Stronger Notification)."
