Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2
$thumbCachePath = "$env:LOCALAPPDATA\Microsoft\Windows\Explorer"
Get-ChildItem -Path $thumbCachePath -Filter "thumbcache_*.db" | Remove-Item -Force -ErrorAction SilentlyContinue
Write-Host "Thumbnail cache cleared."
Start-Process explorer
Write-Host "Explorer restarted."
