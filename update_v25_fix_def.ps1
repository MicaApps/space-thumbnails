
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Stop-Process -Name dllhost -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

Remove-Item "C:\\Users\\Public\\space_thumbnails_debug.log" -ErrorAction SilentlyContinue

Write-Host "Building v25 (DEF File Exports) - Attempt 3...";
$targetDir = "target_temp_debug_v25"
$defFile = "$PWD\crates\windows\space_thumbnails_windows.def"

# Build DLL with DEF file
cd crates/windows
# Use absolute path for DEF file to avoid LNK1104
cargo rustc --release --target-dir "../../$targetDir" --crate-type cdylib -- -C link-arg="/DEF:$defFile"
$buildStatus = $?
cd ../..

if ($buildStatus) {
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
    
    # Update Registry
    $regFile = "$PWD\update_v25.reg"
    Start-Process reg.exe -ArgumentList "import", "`"$regFile`"" -Wait
    
    Write-Host "Registry updated with v25 DLL path."
    
    # Clear Thumbnail Cache
    Get-ChildItem "$env:LOCALAPPDATA\Microsoft\Windows\Explorer\thumbcache_*.db" | Remove-Item -Force -ErrorAction SilentlyContinue
    
    Start-Process explorer.exe
} else {
    Write-Host "Build failed."
    Start-Process explorer.exe
}
