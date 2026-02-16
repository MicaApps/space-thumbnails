
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Stop-Process -Name dllhost -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

Remove-Item "C:\\Users\\Public\\space_thumbnails_debug.log" -ErrorAction SilentlyContinue

Write-Host "Re-Applying Registry Fix (v26)...";
$targetDir = "target_temp_debug_v26"
$releaseDir = "$PWD\$targetDir\release"
$dllPath = "$releaseDir\space_thumbnails_windows.dll"

if (!(Test-Path $dllPath)) {
    Write-Host "Error: DLL not found at $dllPath"
    exit
}

# Define CLSIDs
$clsid_obj = "{650a0a50-3a8c-49ca-ba26-13b31965b8ef}"
$clsid_fbx = "{bf2644df-ae9c-4524-8bfd-2d531b837e97}"
$clsid_stp = "{552657d4-0325-4632-9154-116584281359}"
$clsid_step = "{662657d4-0325-4632-9154-116584281360}"
$thumb_provider_iid = "{e357fccd-a995-4576-b01f-234630154e96}"

# Remove HKCU Overrides
Write-Host "Removing HKCU Overrides..."
Remove-Item -Path "HKCU:\Software\Classes\.step" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path "HKCU:\Software\Classes\.stp" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path "HKCU:\Software\Classes\.obj" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path "HKCU:\Software\Classes\.fbx" -Recurse -Force -ErrorAction SilentlyContinue

Remove-Item -Path "HKCU:\Software\Classes\CLSID\$clsid_step" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path "HKCU:\Software\Classes\CLSID\$clsid_stp" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path "HKCU:\Software\Classes\CLSID\$clsid_obj" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path "HKCU:\Software\Classes\CLSID\$clsid_fbx" -Recurse -Force -ErrorAction SilentlyContinue

# Generate Registry File for HKLM (Using Unicode/UTF-16LE)
$regContent = @"
Windows Registry Editor Version 5.00

; .obj association (ThumbnailProvider - Stream)
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\.obj\ShellEx\$thumb_provider_iid]
@="$clsid_obj"

; .fbx association (ThumbnailProvider - Stream)
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\.fbx\ShellEx\$thumb_provider_iid]
@="$clsid_fbx"

; .stp association (ThumbnailFileProvider - File)
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\.stp\ShellEx\$thumb_provider_iid]
@="$clsid_stp"

; .step association (ThumbnailFileProvider - File)
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\.step\ShellEx\$thumb_provider_iid]
@="$clsid_step"

; CLSID Registration - .obj
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\$clsid_obj]
@="SpaceThumbnails OBJ Provider"
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\$clsid_obj\InProcServer32]
@="$($dllPath.Replace('\', '\\'))"
"ThreadingModel"="Apartment"

; CLSID Registration - .fbx
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\$clsid_fbx]
@="SpaceThumbnails FBX Provider"
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\$clsid_fbx\InProcServer32]
@="$($dllPath.Replace('\', '\\'))"
"ThreadingModel"="Apartment"

; CLSID Registration - .stp
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\$clsid_stp]
@="SpaceThumbnails STP Provider"
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\$clsid_stp\InProcServer32]
@="$($dllPath.Replace('\', '\\'))"
"ThreadingModel"="Apartment"

; CLSID Registration - .step
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\$clsid_step]
@="SpaceThumbnails STEP Provider"
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\$clsid_step\InProcServer32]
@="$($dllPath.Replace('\', '\\'))"
"ThreadingModel"="Apartment"
"@

$regFile = "$PWD\update_v26_fixed.reg"
$regContent | Out-File -FilePath $regFile -Encoding Unicode

Write-Host "Importing Registry File..."
Start-Process reg.exe -ArgumentList "import", "`"$regFile`"" -Wait

Write-Host "Registry updated with v26 DLL path (HKLM)."

# Clear Thumbnail Cache
Get-ChildItem "$env:LOCALAPPDATA\Microsoft\Windows\Explorer\thumbcache_*.db" | Remove-Item -Force -ErrorAction SilentlyContinue

Start-Process explorer.exe
