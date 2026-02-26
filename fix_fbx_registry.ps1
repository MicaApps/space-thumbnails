# Fix FBX Registry Association Script
# This script resets the .fbx file association to use the correct SpaceThumbnails FBX handler.

$ErrorActionPreference = "Stop"

Write-Host "Checking .fbx registry settings..."

# Define the correct CLSID for SpaceThumbnails FBX
$fbxClsid = "{BF2644DF-AE9C-4524-8BFD-2D531B837E97}"
$progId = "SpaceThumbnails.FbxFile"

# 1. Create ProgID if missing
Write-Host "Setting up ProgID: $progId"
New-Item -Path "HKCU\Software\Classes\$progId" -Force | Out-Null
New-ItemProperty -Path "HKCU\Software\Classes\$progId" -Name "(default)" -Value "FBX Model File" -Force | Out-Null
New-Item -Path "HKCU\Software\Classes\$progId\CLSID" -Force | Out-Null
New-ItemProperty -Path "HKCU\Software\Classes\$progId\CLSID" -Name "(default)" -Value $fbxClsid -Force | Out-Null

# 2. Reset .fbx extension default
Write-Host "Resetting .fbx default association..."
New-Item -Path "HKCU\Software\Classes\.fbx" -Force | Out-Null
New-ItemProperty -Path "HKCU\Software\Classes\.fbx" -Name "(default)" -Value $progId -Force | Out-Null

# 3. Clear UserChoice (Requires User Intervention or specialized tools usually, but we can try removing the key)
# Note: Windows protects UserChoice with a hash. Deleting it usually resets to "Pick an app".
$userChoicePath = "HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.fbx\UserChoice"
if (Test-Path $userChoicePath) {
    Write-Host "Attempting to remove UserChoice override (this might fail due to permissions)..."
    try {
        Remove-Item -Path $userChoicePath -Force -Recurse
        Write-Host "UserChoice removed. Windows will ask for default app next time."
    } catch {
        Write-Warning "Could not remove UserChoice key. You may need to manually reset 'Open With' settings for .fbx files."
        Write-Warning "Error: $_"
    }
}

# 4. Refresh Explorer
Write-Host "Restarting Explorer to apply changes..."
Stop-Process -Name explorer -Force
Start-Process explorer

Write-Host "Done. Please check if FBX thumbnails are now working correctly."
