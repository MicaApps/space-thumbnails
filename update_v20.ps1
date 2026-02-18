
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Stop-Process -Name dllhost -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

Remove-Item "C:\\Users\\Public\\space_thumbnails_debug.log" -ErrorAction SilentlyContinue

Write-Host "Building v20 (CLI RESTORED)...";
$targetDir = "target_temp_debug_v20"

# Build DLL and CLI
cargo build --release --target-dir $targetDir -p space-thumbnails-windows -p space-thumbnails-cli

if ($?) {
    Write-Host "Build successful."
    
    $releaseDir = "$PWD\$targetDir\release"
    
    # Copy tools folder to release directory
    $toolsDest = "$releaseDir\tools"
    if (Test-Path $toolsDest) { Remove-Item $toolsDest -Recurse -Force }
    Copy-Item "tools" -Destination $releaseDir -Recurse
    
    Write-Host "Tools copied to $toolsDest"

    # Register DLL
    $dllPath = "$releaseDir\space_thumbnails_windows.dll"
    
    # Unregister old (optional, but good practice)
    # regsvr32.exe /u /s $dllPath 
    
    # Register new
    Start-Process regsvr32.exe -ArgumentList "/s", "`"$dllPath`"" -Wait
    
    Write-Host "Registered v20 DLL at $dllPath"
    
    # Force update registry just in case
    # We can use our update_registry.ps1 logic but pointing to v20
    $regContent = @"
Windows Registry Editor Version 5.00

[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\{650a0a50-3a8c-49ca-ba26-13b31965b8ef}\InProcServer32]
@="$($dllPath.Replace('\', '\\'))"
"ThreadingModel"="Apartment"

[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\{bf2644df-ae9c-4524-8bfd-2d531b837e97}\InProcServer32]
@="$($dllPath.Replace('\', '\\'))"
"ThreadingModel"="Apartment"

[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\{95305943-4886-4f4c-b016-8c0125722421}\InProcServer32]
@="$($dllPath.Replace('\', '\\'))"
"ThreadingModel"="Apartment"

[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\{d456070f-1502-4014-9975-ca26e95638d2}\InProcServer32]
@="$($dllPath.Replace('\', '\\'))"
"ThreadingModel"="Apartment"
"@
    $regFile = "$PWD\update_v20.reg"
    $regContent | Out-File -FilePath $regFile -Encoding UTF8
    
    Start-Process reg.exe -ArgumentList "import", "`"$regFile`"" -Wait
    
    Start-Process explorer.exe
} else {
    Write-Host "Build failed."
    Start-Process explorer.exe
}
