
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Stop-Process -Name dllhost -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

Remove-Item "C:\\Users\\Public\\space_thumbnails_debug.log" -ErrorAction SilentlyContinue

Write-Host "Building v22 (Registry & Interface Fix)...";
$targetDir = "target_temp_debug_v22"

# Build DLL only (CLI is fine)
cargo build --release --target-dir $targetDir -p space-thumbnails-windows

if ($?) {
    Write-Host "Build successful."
    
    $releaseDir = "$PWD\$targetDir\release"
    
    # Copy tools folder to release directory
    $toolsDest = "$releaseDir\tools"
    if (Test-Path $toolsDest) { Remove-Item $toolsDest -Recurse -Force }
    Copy-Item "tools" -Destination $releaseDir -Recurse
    
    # Also copy CLI (since we rebuilt DLL in new dir, we need CLI there too)
    Copy-Item "target_temp_debug_v21\release\space-thumbnails-cli.exe" -Destination "$releaseDir\space-thumbnails-cli.exe"

    Write-Host "Tools and CLI copied to $toolsDest"

    # Register DLL
    $dllPath = "$releaseDir\space_thumbnails_windows.dll"
    
    Start-Process regsvr32.exe -ArgumentList "/s", "`"$dllPath`"" -Wait
    
    Write-Host "Registered v22 DLL at $dllPath"
    
    # Update Registry Associations (CRITICAL STEP)
    $regContent = @"
Windows Registry Editor Version 5.00

; .step association
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\.step\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}]
@="{650a0a50-3a8c-49ca-ba26-13b31965b8ef}"

; .stp association
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\.stp\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}]
@="{bf2644df-ae9c-4524-8bfd-2d531b837e97}"

; CLSID registration for .step
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\{650a0a50-3a8c-49ca-ba26-13b31965b8ef}\InProcServer32]
@="$($dllPath.Replace('\', '\\'))"
"ThreadingModel"="Apartment"

; CLSID registration for .stp
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\{bf2644df-ae9c-4524-8bfd-2d531b837e97}\InProcServer32]
@="$($dllPath.Replace('\', '\\'))"
"ThreadingModel"="Apartment"
"@
    $regFile = "$PWD\update_v22.reg"
    $regContent | Out-File -FilePath $regFile -Encoding UTF8
    
    Start-Process reg.exe -ArgumentList "import", "`"$regFile`"" -Wait
    
    # Clear Thumbnail Cache
    Write-Host "Clearing Thumbnail Cache..."
    Get-ChildItem "$env:LOCALAPPDATA\Microsoft\Windows\Explorer\thumbcache_*.db" | Remove-Item -Force -ErrorAction SilentlyContinue
    
    Start-Process explorer.exe
} else {
    Write-Host "Build failed."
    Start-Process explorer.exe
}
