# Build DLL and CLI, then copy to control-panel output directory
$ErrorActionPreference = "Stop"

$projectRoot = $PSScriptRoot
$targetDir = Join-Path $projectRoot "control-panel\bin\x64\Release\net8.0-windows10.0.19041.0"

Write-Host "Building Rust project in release mode..."
cargo build --release

# Ensure target directory exists
if (-not (Test-Path $targetDir)) {
    Write-Host "Creating target directory: $targetDir"
    New-Item -ItemType Directory -Path $targetDir | Out-Null
}

$dllSrc = Join-Path $projectRoot "target\release\space_thumbnails_windows.dll"
$cliSrc = Join-Path $projectRoot "target\release\space-thumbnails-cli.exe"

$dllDest = Join-Path $targetDir "space_thumbnails_windows.dll"
$cliDest = Join-Path $targetDir "space-thumbnails-cli.exe"

Write-Host "Copying DLL to $dllDest..."
Copy-Item -Path $dllSrc -Destination $dllDest -Force

Write-Host "Copying CLI to $cliDest..."
Copy-Item -Path $cliSrc -Destination $cliDest -Force

Write-Host "Done! DLL and CLI have been generated and copied to $targetDir"
