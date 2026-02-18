
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Stop-Process -Name dllhost -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

Remove-Item "C:\\Users\\Public\\space_thumbnails_debug.log" -ErrorAction SilentlyContinue

Write-Host "Building v16 (Stream Debug)..."
cargo build --release --target-dir target_temp_debug_v10 -p space-thumbnails-windows

if ($?) {
    Write-Host "Build successful."
    Start-Process explorer.exe
} else {
    Write-Host "Build failed."
    Start-Process explorer.exe
}
