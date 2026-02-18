$ErrorActionPreference = "Stop"

# Use dynamic lookup to avoid encoding issues with hardcoded non-ASCII paths
$FileName = "HLD60-J1-G7.STEP.step"
$SearchRoot = "D:\Users\Shomn\OneDrive - MSFT\Work"

Write-Host "Searching for $FileName in $SearchRoot..."
$FileObj = Get-ChildItem -Path $SearchRoot -Recurse -Filter $FileName -ErrorAction SilentlyContinue | Select-Object -First 1

if ($null -eq $FileObj) {
    Write-Error "Could not find file: $FileName"
    exit 1
}

$InputFile = $FileObj.FullName
$OutputFile = "$env:TEMP\test_manual_v31.obj"
$BatFile = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\tools\step2obj.bat"
$LogFile = "$OutputFile.log"

Write-Host "Found Input: $InputFile"
Write-Host "Output: $OutputFile"
Write-Host "LogFile: $LogFile"

if (Test-Path $OutputFile) { Remove-Item $OutputFile -Force }
if (Test-Path $LogFile) { Remove-Item $LogFile -Force }

# Execute the CLI directly
$CliExe = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\target_temp_debug_v29\release\space-thumbnails-cli.exe"
$CliOutput = "$env:TEMP\test_cli_manual.png"
if (Test-Path $CliOutput) { Remove-Item $CliOutput -Force }

$ProcessArgs = "`"$InputFile`" `"$CliOutput`" --width 256 --height 256 --api default"
Write-Host "Executing CLI: $CliExe $ProcessArgs"

Start-Process -FilePath $CliExe -ArgumentList $ProcessArgs -Wait -NoNewWindow

if (Test-Path $CliOutput) {
    Write-Host "Success! PNG file generated."
    $item = Get-Item $CliOutput
    Write-Host "Size: $($item.Length) bytes"
} else {
    Write-Error "Failed to generate PNG file."
}

if (Test-Path $LogFile) {
    Write-Host "`n=== Log File Content ==="
    Get-Content $LogFile
    Write-Host "========================"
} else {
    Write-Host "No log file found at $LogFile"
}
