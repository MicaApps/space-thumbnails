
$clsidHandler = "{662657D4-0325-4632-9154-116584281360}"
$iidThumbnail = "{E357FCCD-A995-4576-B01F-234630154E96}"
$progId = "STEP ISO 10303"

Write-Host "Registering Thumbnail Provider for ProgID: $progId"

$hklmProgId = "Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Classes\$progId"
$hklmShellEx = "$hklmProgId\ShellEx"
$hklmThumb = "$hklmShellEx\$iidThumbnail"

# Create ShellEx if missing
if (-not (Test-Path $hklmShellEx)) {
    New-Item -Path $hklmProgId -Name "ShellEx" -Force | Out-Null
}

# Create Thumbnail Provider Key
New-Item -Path $hklmShellEx -Name $iidThumbnail -Force | Out-Null
Set-ItemProperty -Path $hklmThumb -Name "(default)" -Value $clsidHandler

Write-Host "Registration complete for $progId"
Write-Host "Please restart Explorer to apply changes."

Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2
Start-Process explorer.exe
