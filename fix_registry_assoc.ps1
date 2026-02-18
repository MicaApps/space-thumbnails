
$ErrorActionPreference = "SilentlyContinue"

# Define CLSIDs
$clsid_step = "{650a0a50-3a8c-49ca-ba26-13b31965b8ef}"
$clsid_stp = "{bf2644df-ae9c-4524-8bfd-2d531b837e97}"
$clsid_obj = "{95305943-4886-4f4c-b016-8c0125722421}"
$clsid_fbx = "{d456070f-1502-4014-9975-ca26e95638d2}"
$thumb_provider_iid = "{e357fccd-a995-4576-b01f-234630154e96}"

Write-Host "Fixing Registry Associations..."

# Helper function to set association
function Set-Assoc {
    param($ext, $clsid)
    $path = "HKLM:\SOFTWARE\Classes\$ext\ShellEx\$thumb_provider_iid"
    
    # Create key if not exists
    if (!(Test-Path $path)) {
        New-Item -Path $path -Force | Out-Null
    }
    
    # Set default value
    Set-Item -Path $path -Value $clsid
    Write-Host "Set $ext association to $clsid (HKLM)"
    
    # Clean HKCU (User overrides can cause issues)
    $userPath = "HKCU:\Software\Classes\$ext\ShellEx\$thumb_provider_iid"
    if (Test-Path $userPath) {
        Remove-Item $userPath -Force
        Write-Host "Removed user override for $ext (HKCU)"
    }
}

Set-Assoc -ext ".step" -clsid $clsid_step
Set-Assoc -ext ".stp" -clsid $clsid_stp
Set-Assoc -ext ".obj" -clsid $clsid_obj
Set-Assoc -ext ".fbx" -clsid $clsid_fbx

# Verify DLL path registration (just to be sure)
$dllPath = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\target_temp_debug_v21\release\space_thumbnails_windows.dll"
if (!(Test-Path $dllPath)) {
    Write-Host "ERROR: DLL not found at $dllPath" -ForegroundColor Red
} else {
    Write-Host "DLL verified at $dllPath"
}

# Restart Explorer to apply changes
Stop-Process -Name explorer -Force
Start-Sleep -Seconds 1
Start-Process explorer.exe

Write-Host "Registry updated and Explorer restarted."
