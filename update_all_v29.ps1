$ErrorActionPreference = "Stop"

Write-Host "Stopping Explorer and DLLHost to unlock files..."
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Stop-Process -Name dllhost -Force -ErrorAction SilentlyContinue
# Also kill CLI if running
Stop-Process -Name space-thumbnails-cli -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

# Clean up SpaceThumbnails Cache to force regeneration with new material settings
$cachePath = "$env:TEMP\SpaceThumbnailsCache"
if (Test-Path $cachePath) {
    Write-Host "Cleaning up old thumbnail cache at $cachePath..."
    Remove-Item $cachePath -Recurse -Force -ErrorAction SilentlyContinue
}

$targetDir = "target_temp_debug_v29"
$releaseDir = "$PWD\$targetDir\release"

if (!(Test-Path $releaseDir)) {
    New-Item -ItemType Directory -Path $releaseDir -Force
}

Write-Host "Building Core DLL (release)..."
cargo build --release --package space-thumbnails-windows
if ($LASTEXITCODE -ne 0) { Write-Error "DLL build failed"; exit 1 }

Write-Host "Building CLI (release)..."
cargo build --release --bin space-thumbnails-cli
if ($LASTEXITCODE -ne 0) { Write-Error "CLI build failed"; exit 1 }

Write-Host "Deploying files to $releaseDir..."
Copy-Item "target\release\space_thumbnails_windows.dll" -Destination "$releaseDir\space_thumbnails_windows.dll" -Force
Copy-Item "target\release\space-thumbnails-cli.exe" -Destination "$releaseDir\space-thumbnails-cli.exe" -Force
Copy-Item "target\release\Loading.png" -Destination "$releaseDir\Loading.png" -Force

Write-Host "Deploying tools..."
if (Test-Path "$releaseDir\tools") {
    Remove-Item "$releaseDir\tools" -Recurse -Force
}
Copy-Item "tools" -Destination "$releaseDir" -Recurse -Force

# Optional: Register the DLL if path changed (but if we overwrite, it's fine)
Write-Host "Registering DLL..."
regsvr32 /s "$releaseDir\space_thumbnails_windows.dll"

Write-Host "Configuring registry to disable thumbnail shadows..."
$exts = @(".step", ".stp")
foreach ($ext in $exts) {
    # 1. HKCR\.ext\Treatment = 0
    $path = "Registry::HKEY_CLASSES_ROOT\$ext"
    if (!(Test-Path $path)) { New-Item -Path $path -Force | Out-Null }
    try {
        Set-ItemProperty -Path $path -Name "Treatment" -Value 0 -Type DWord -Force -ErrorAction SilentlyContinue
        Write-Host "Set HKCR\$ext\Treatment = 0"
    } catch {
        Write-Warning "Failed to set HKCR\$ext\Treatment: $_"
    }

    # 2. HKCR\SystemFileAssociations\.ext\Treatment = 0
    $sysPath = "Registry::HKEY_CLASSES_ROOT\SystemFileAssociations\$ext"
    if (!(Test-Path $sysPath)) { New-Item -Path $sysPath -Force | Out-Null }
    try {
        Set-ItemProperty -Path $sysPath -Name "Treatment" -Value 0 -Type DWord -Force -ErrorAction SilentlyContinue
        Write-Host "Set HKCR\SystemFileAssociations\$ext\Treatment = 0"
    } catch {
        Write-Warning "Failed to set HKCR\SystemFileAssociations\$ext\Treatment: $_"
    }
}

Write-Host "Restarting Explorer..."
Start-Process explorer

Write-Host "Update Complete (v29 - Background Generation)."
