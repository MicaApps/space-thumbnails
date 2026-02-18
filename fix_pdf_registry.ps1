# Stop Explorer
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

# Register the DLL (Standard)
$dllPath = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\target\release\space_thumbnails_windows.dll"
regsvr32 /s $dllPath

# Add the MSEdgePDF Thumbnail Provider Key manually
# CLSID for IThumbnailProvider is {e357fccd-a995-4576-b01f-234630154e96}
# Our PDF Provider CLSID is {102657d4-0325-4632-9154-116584281399}

$msePath = "Registry::HKEY_CLASSES_ROOT\MSEdgePDF\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}"
if (-not (Test-Path $msePath)) {
    New-Item -Path $msePath -Force | Out-Null
}
Set-Item -Path $msePath -Value "{102657d4-0325-4632-9154-116584281399}"

Write-Host "Registered PDF Thumbnail Provider for MSEdgePDF"

# Start Explorer
Start-Process explorer
