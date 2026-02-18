
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Stop-Process -Name dllhost -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

Remove-Item "C:\Users\Public\space_thumbnails_debug.log" -ErrorAction SilentlyContinue

Write-Host "Building v15 (Magenta)..."
cargo build --release --target-dir target_temp_debug_v10 -p space-thumbnails-windows

if ($?) {
    Write-Host "Build successful."
    
    # Create the test file if not exists
    $testFile = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\assets\LGQGJ-00-00 轮毂抛光机总装.STEP"
    if (-not (Test-Path $testFile)) {
        Copy-Item "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\assets\test.step" $testFile
        Write-Host "Created test file: $testFile"
    }
    
    Start-Process explorer.exe
} else {
    Write-Host "Build failed."
    Start-Process explorer.exe
}
