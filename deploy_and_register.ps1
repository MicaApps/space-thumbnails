$projectRoot = "d:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6"
$src = "$projectRoot\target\release\space_thumbnails_windows.dll"
$srcCli = "$projectRoot\target\release\space-thumbnails-cli.exe"
$dest = "d:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\control-panel\bin\x64\Release\net8.0-windows10.0.19041.0\space_thumbnails_windows.dll"
$destCli = "d:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\control-panel\bin\x64\Release\net8.0-windows10.0.19041.0\space-thumbnails-cli.exe"

Write-Host "Deploying DLL from $src to $dest..."

# Check if source exists
if (-not (Test-Path $src)) {
    Write-Host "Error: Source DLL not found at $src"
    exit 1
}

# Check if CLI source exists
if (-not (Test-Path $srcCli)) {
    Write-Host "Error: Source CLI not found at $srcCli"
    exit 1
}

# Unregister first to release lock
Write-Host "Unregistering old DLL..."
Start-Process "regsvr32" -ArgumentList "/u /s `"$dest`"" -Wait

# Stop explorer to release file locks
Write-Host "Stopping Explorer..."
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

# Copy DLL
try {
    Copy-Item -Path $src -Destination $dest -Force -ErrorAction Stop
    Write-Host "DLL Copy success"
} catch {
    Write-Host "DLL Copy failed: $_"
    exit 1
}

# Copy CLI
try {
    Copy-Item -Path $srcCli -Destination $destCli -Force -ErrorAction Stop
    Write-Host "CLI Copy success"
} catch {
    Write-Host "CLI Copy failed: $_"
    # Don't exit here, maybe DLL is enough for some cases, but strictly we need both
    exit 1
}

$absDest = (Resolve-Path $dest).Path
Write-Host "Registering $absDest..."
Start-Process regsvr32.exe -ArgumentList "/s `"$absDest`"" -Wait

Write-Host "Registration command completed."
Start-Process explorer
Write-Host "Explorer restarted."
