
# Close Explorer and COM Surrogate
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Stop-Process -Name dllhost -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

# Define CLSIDs
$clsid_obj = "{650a0a50-3a8c-49ca-ba26-13b31965b8ef}"
$clsid_fbx = "{bf2644df-ae9c-4524-8bfd-2d531b837e97}"
$clsid_stp = "{552657d4-0325-4632-9154-116584281359}"
$clsid_step = "{662657d4-0325-4632-9154-116584281360}"
$thumb_provider_iid = "{e357fccd-a995-4576-b01f-234630154e96}"

# Define DLL Path (v24)
$dllPath = "$PWD\target_temp_debug_v24\release\space_thumbnails_windows.dll"

if (!(Test-Path $dllPath)) {
    Write-Host "Error: v24 DLL not found at $dllPath"
    exit 1
}

Write-Host "Using DLL: $dllPath"

# Function to Set Registry Key
function Set-RegKey {
    param($path, $name, $value, $type="String")
    if (!(Test-Path $path)) {
        New-Item -Path $path -Force | Out-Null
    }
    if ($name -eq "(Default)") {
        Set-Item -Path $path -Value $value
    } else {
        Set-ItemProperty -Path $path -Name $name -Value $value -Type $type
    }
    Write-Host "Set $path\$name = $value"
}

# 1. Update File Associations
# .obj
Set-RegKey "HKLM:\SOFTWARE\Classes\.obj\ShellEx\$thumb_provider_iid" "(Default)" $clsid_obj
# .fbx
Set-RegKey "HKLM:\SOFTWARE\Classes\.fbx\ShellEx\$thumb_provider_iid" "(Default)" $clsid_fbx
# .stp
Set-RegKey "HKLM:\SOFTWARE\Classes\.stp\ShellEx\$thumb_provider_iid" "(Default)" $clsid_stp
# .step
Set-RegKey "HKLM:\SOFTWARE\Classes\.step\ShellEx\$thumb_provider_iid" "(Default)" $clsid_step

# 2. Update CLSID Registrations
# .obj
Set-RegKey "HKLM:\SOFTWARE\Classes\CLSID\$clsid_obj\InProcServer32" "(Default)" $dllPath
Set-RegKey "HKLM:\SOFTWARE\Classes\CLSID\$clsid_obj\InProcServer32" "ThreadingModel" "Apartment"

# .fbx
Set-RegKey "HKLM:\SOFTWARE\Classes\CLSID\$clsid_fbx\InProcServer32" "(Default)" $dllPath
Set-RegKey "HKLM:\SOFTWARE\Classes\CLSID\$clsid_fbx\InProcServer32" "ThreadingModel" "Apartment"

# .stp
Set-RegKey "HKLM:\SOFTWARE\Classes\CLSID\$clsid_stp\InProcServer32" "(Default)" $dllPath
Set-RegKey "HKLM:\SOFTWARE\Classes\CLSID\$clsid_stp\InProcServer32" "ThreadingModel" "Apartment"

# .step
Set-RegKey "HKLM:\SOFTWARE\Classes\CLSID\$clsid_step\InProcServer32" "(Default)" $dllPath
Set-RegKey "HKLM:\SOFTWARE\Classes\CLSID\$clsid_step\InProcServer32" "ThreadingModel" "Apartment"

# 3. Clean up User Overrides (HKCU)
$user_extensions = @(".obj", ".fbx", ".stp", ".step")
foreach ($ext in $user_extensions) {
    $userPath = "HKCU:\Software\Classes\$ext\ShellEx\$thumb_provider_iid"
    if (Test-Path $userPath) {
        Remove-Item $userPath -Force
        Write-Host "Removed user override for $ext (HKCU)"
    }
}

# 4. Clear Cache and Restart
Write-Host "Clearing Thumbnail Cache..."
Get-ChildItem "$env:LOCALAPPDATA\Microsoft\Windows\Explorer\thumbcache_*.db" | Remove-Item -Force -ErrorAction SilentlyContinue

Start-Process explorer.exe
Write-Host "Done."
