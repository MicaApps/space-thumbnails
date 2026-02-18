if (!([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Start-Process powershell.exe "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`"" -Verb RunAs
    exit
}

$clsid_step = "{662657D4-0325-4632-9154-116584281360}"
$clsid_stp = "{552657D4-0325-4632-9154-116584281359}"
$dll_path = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\target_temp_debug_v10\release\space_thumbnails_windows.dll"

Write-Host "Updating registry to point to: $dll_path"

try {
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid_step\InProcServer32" -Name "(default)" -Value $dll_path -ErrorAction Stop
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid_stp\InProcServer32" -Name "(default)" -Value $dll_path -ErrorAction Stop
    Write-Host "Registry updated successfully."
} catch {
    Write-Host "Error updating registry: $_"
}

# Also restart explorer to ensure it picks up the change (although registry change usually immediate, DLL unloading needs restart)
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Start-Process explorer.exe
