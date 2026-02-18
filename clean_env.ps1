
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Stop-Process -Name dllhost -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

# Remove HKCU file associations for .step/.stp to force HKLM usage
Remove-Item -Path "Registry::HKEY_CURRENT_USER\Software\Classes\.step\ShellEx" -Recurse -ErrorAction SilentlyContinue
Remove-Item -Path "Registry::HKEY_CURRENT_USER\Software\Classes\.stp\ShellEx" -Recurse -ErrorAction SilentlyContinue

# Clear Thumbnail Cache
$thumbCache = "$env:LOCALAPPDATA\Microsoft\Windows\Explorer"
Get-ChildItem -Path $thumbCache -Filter "thumbcache_*.db" -Recurse | Remove-Item -Force -ErrorAction SilentlyContinue

# Delete debug log
Remove-Item "C:\\Users\\Public\\space_thumbnails_debug.log" -ErrorAction SilentlyContinue

Write-Host "Environment cleaned. Restarting Explorer..."
Start-Process explorer.exe
