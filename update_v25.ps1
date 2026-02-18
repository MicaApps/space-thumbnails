
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Stop-Process -Name dllhost -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

Remove-Item "C:\\Users\\Public\\space_thumbnails_debug.log" -ErrorAction SilentlyContinue

Write-Host "Building v25 (DEF File Exports)...";
$targetDir = "target_temp_debug_v25"

# Build DLL with DEF file
cd crates/windows
cargo rustc --release --target-dir "../../$targetDir" --crate-type cdylib -- -C link-arg=/DEF:space_thumbnails_windows.def
cd ../..

if ($?) {
    Write-Host "Build successful."
    
    $releaseDir = "$PWD\$targetDir\release"
    
    # Copy tools folder to release directory
    $toolsDest = "$releaseDir\tools"
    if (Test-Path $toolsDest) { Remove-Item $toolsDest -Recurse -Force }
    Copy-Item "tools" -Destination $releaseDir -Recurse
    
    # Copy CLI
    Copy-Item "target_temp_debug_v21\release\space-thumbnails-cli.exe" -Destination "$releaseDir\space-thumbnails-cli.exe"

    Write-Host "Tools and CLI copied to $toolsDest"

    # Register DLL
    $dllPath = "$releaseDir\space_thumbnails_windows.dll"
    
    Start-Process regsvr32.exe -ArgumentList "/s", "`"$dllPath`"" -Wait
    
    Write-Host "Registered v25 DLL at $dllPath"
    
    # Update Registry (same as v24)
    # Define Correct CLSIDs
    $clsid_obj = "{650a0a50-3a8c-49ca-ba26-13b31965b8ef}"
    $clsid_fbx = "{bf2644df-ae9c-4524-8bfd-2d531b837e97}"
    $clsid_stp = "{552657d4-0325-4632-9154-116584281359}"
    $clsid_step = "{662657d4-0325-4632-9154-116584281360}"
    $thumb_provider_iid = "{e357fccd-a995-4576-b01f-234630154e96}"
    
    # Generate Registry File
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
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\$clsid_obj\InProcServer32]
@="$($dllPath.Replace('\', '\\'))"
"ThreadingModel"="Apartment"

; CLSID Registration - .fbx
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\$clsid_fbx\InProcServer32]
@="$($dllPath.Replace('\', '\\'))"
"ThreadingModel"="Apartment"

; CLSID Registration - .stp
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\$clsid_stp\InProcServer32]
@="$($dllPath.Replace('\', '\\'))"
"ThreadingModel"="Apartment"

; CLSID Registration - .step
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\$clsid_step\InProcServer32]
@="$($dllPath.Replace('\', '\\'))"
"ThreadingModel"="Apartment"
"@
    
    $regFile = "$PWD\update_v25.reg"
    $regContent | Out-File -FilePath $regFile -Encoding UTF8
    
    Start-Process reg.exe -ArgumentList "import", "`"$regFile`"" -Wait
    
    Write-Host "Registry updated with v25 DLL path."
    
    # Clear Thumbnail Cache
    Get-ChildItem "$env:LOCALAPPDATA\Microsoft\Windows\Explorer\thumbcache_*.db" | Remove-Item -Force -ErrorAction SilentlyContinue
    
    Start-Process explorer.exe
} else {
    Write-Host "Build failed."
    Start-Process explorer.exe
}
