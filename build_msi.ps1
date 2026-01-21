$ErrorActionPreference = "Stop"
# Try standard path first
$wixBin = "C:\Program Files (x86)\WiX Toolset v3.11\bin"
if (-not (Test-Path $wixBin)) {
    # Try finding it in Program Files (x86)
    $heat = Get-ChildItem "C:\Program Files (x86)" -Filter "heat.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -eq $heat) {
         # Try Program Files
         $heat = Get-ChildItem "C:\Program Files" -Filter "heat.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
    }
    
    if ($null -eq $heat) {
        Write-Error "Could not find WiX Toolset (heat.exe)."
    }
    $wixBin = $heat.DirectoryName
}
Write-Host "Using WiX at: $wixBin"

$heat = "$wixBin\heat.exe"
$candle = "$wixBin\candle.exe"
$light = "$wixBin\light.exe"

# 1. Prepare dist for Heat (Exclude DLL to avoid duplication with manual component)
$dllPath = "dist\space_thumbnails_windows_dll.dll"
$dllTemp = "space_thumbnails_windows_dll.dll"
if (Test-Path $dllPath) { Move-Item $dllPath $dllTemp -Force }

# 2. Harvest
# -dr INSTALLFOLDER: Files go into this directory
# -srd: Harvest content of dist
# -cg ProductComponents: Group name
# -sw5150: Suppress self-reg warning (we handle it manually)
Write-Host "Harvesting files..."
& $heat dir "dist" -cg ProductComponents -dr INSTALLFOLDER -srd -gg -sfrag -template fragment -out files.wxs -var var.SourceDir

# 3. Compile
Write-Host "Compiling..."
& $candle product.wxs files.wxs -dSourceDir="dist" -arch x64

# 4. Link
# -sice:ICE61: Suppress upgrade code warning if any (optional)
# -sw1076: ICE61 warning
# -sice:ICE03: Invalid Language Id (common with auto-harvested files)
Write-Host "Linking..."
& $light product.wixobj files.wixobj -o SpaceThumbnails_Setup.msi -ext WixUIExtension -sw1076 -sice:ICE03

# 5. Restore DLL
if (Test-Path $dllTemp) { Move-Item $dllTemp $dllPath -Force }

Write-Host "MSI Created: SpaceThumbnails_Setup.msi"