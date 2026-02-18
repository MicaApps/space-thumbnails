$cachePath = "$env:LOCALAPPDATA\space-thumbnails\cache"
if (Test-Path $cachePath) {
    Write-Host "Cleaning lock files in $cachePath"
    Get-ChildItem "$cachePath\*.lock" | Remove-Item -Force -ErrorAction SilentlyContinue
    Write-Host "Cleaned."
} else {
    Write-Host "Cache path not found: $cachePath"
}
