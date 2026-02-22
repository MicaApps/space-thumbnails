Write-Host "Closing Explorer..."
taskkill /f /im explorer.exe

Write-Host "Cleaning Icon Cache Database..."
$iconCachePath = "$env:LOCALAPPDATA\Microsoft\Windows\Explorer"
Get-ChildItem -Path $iconCachePath -Filter "iconcache*" -Recurse | Remove-Item -Force -ErrorAction SilentlyContinue
Get-ChildItem -Path $iconCachePath -Filter "thumbcache*" -Recurse | Remove-Item -Force -ErrorAction SilentlyContinue

Write-Host "Restarting Explorer..."
Start-Process explorer.exe

Write-Host "Icon cache rebuilt successfully."
