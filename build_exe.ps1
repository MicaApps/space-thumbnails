Write-Host "Compiling Bundle..."
& "C:\Program Files (x86)\WiX Toolset v3.14\bin\candle.exe" bundle.wxs -ext WixBalExtension
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Linking Bundle..."
& "C:\Program Files (x86)\WiX Toolset v3.14\bin\light.exe" bundle.wixobj -o SpaceThumbnails_Setup.exe -ext WixBalExtension
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "EXE Created: SpaceThumbnails_Setup.exe"
