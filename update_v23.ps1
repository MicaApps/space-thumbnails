
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Stop-Process -Name dllhost -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

Remove-Item "C:\\Users\\Public\\space_thumbnails_debug.log" -ErrorAction SilentlyContinue

Write-Host "Building v23 (ClassFactory Logging)...";
$targetDir = "target_temp_debug_v23"

# Build DLL only
cargo build --release --target-dir $targetDir -p space-thumbnails-windows

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
    
    Write-Host "Registered v23 DLL at $dllPath"
    
    # Re-import Registry just in case
    $regFile = "$PWD\update_v22.reg"
    Start-Process reg.exe -ArgumentList "import", "`"$regFile`"" -Wait
    
    # Clear Thumbnail Cache again
    Get-ChildItem "$env:LOCALAPPDATA\Microsoft\Windows\Explorer\thumbcache_*.db" | Remove-Item -Force -ErrorAction SilentlyContinue
    
    Start-Process explorer.exe
} else {
    Write-Host "Build failed."
    Start-Process explorer.exe
}
