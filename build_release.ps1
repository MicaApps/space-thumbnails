# Build Release Script for Space Thumbnails
$ErrorActionPreference = "Stop"

$projectRoot = $PSScriptRoot
$distDir = Join-Path $projectRoot "dist"

# Clean dist
if (Test-Path $distDir) { Remove-Item $distDir -Recurse -Force }
New-Item -ItemType Directory -Path $distDir | Out-Null

Write-Host "Collecting files..."

# 1. Control Panel (Win32 App)
$cpBuildDir = "control-panel\bin\x64\Release\net8.0-windows10.0.19041.0"
Copy-Item "$cpBuildDir\*" -Destination $distDir -Recurse -Force

# 2. Rust Artifacts
Copy-Item "target\release\space_thumbnails_windows_dll.dll" -Destination $distDir -Force
Copy-Item "target\release\space-thumbnails-cli.exe" -Destination $distDir -Force

# 3. Tools (Python & Scripts)
$toolsDest = Join-Path $distDir "tools"
if (-not (Test-Path $toolsDest)) {
    New-Item -ItemType Directory -Path $toolsDest | Out-Null
}
Copy-Item "tools\python" -Destination $toolsDest -Recurse -Force
Copy-Item "tools\step2obj.bat" -Destination $toolsDest -Force
Copy-Item "tools\step2obj_occ.py" -Destination $toolsDest -Force

# 4. Clean up PDBs (optional)
Get-ChildItem $distDir -Filter "*.pdb" -Recurse | Remove-Item

Write-Host "Build complete. Output: $distDir"
Write-Host "You can now use Inno Setup to compile the installer from this directory."