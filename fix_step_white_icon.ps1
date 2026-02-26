$ErrorActionPreference = "Continue"

# Define CLSIDs
$step_clsid = "{662657d4-0325-4632-9154-116584281360}"
$stp_clsid = "{552657d4-0325-4632-9154-116584281359}"
$shellex_guid = "{E357FCCD-A995-4576-B01F-234630154E96}"

# Function to set registry key
function Set-RegKey {
    param (
        [string]$Path,
        [string]$Name,
        [string]$Value
    )
    if (!(Test-Path $Path)) {
        New-Item -Path $Path -Force | Out-Null
    }
    Set-ItemProperty -Path $Path -Name $Name -Value $Value
    Write-Host "Set $Path\$Name to $Value"
}

Write-Host "--- Fixing .step Association ---"

# 1. Ensure .step points to SpaceThumbnails.StepFile in HKCU
# This fixes the "white paper" icon issue by giving it a valid ProgID.
Set-RegKey "HKCU:\Software\Classes\.step" "(default)" "SpaceThumbnails.StepFile"

# 2. Ensure SpaceThumbnails.StepFile is correctly registered in HKCU
# It needs a CLSID (optional but good practice) and ShellEx
Set-RegKey "HKCU:\Software\Classes\SpaceThumbnails.StepFile" "(default)" "STEP File"
Set-RegKey "HKCU:\Software\Classes\SpaceThumbnails.StepFile\ShellEx\$shellex_guid" "(default)" $step_clsid

# 3. Handle .STEP (uppercase) just in case
Set-RegKey "HKCU:\Software\Classes\.STEP" "(default)" "SpaceThumbnails.StepFile"

# 4. Force restart Explorer
Write-Host "Restarting Explorer..."
Stop-Process -Name explorer -Force
Start-Sleep -Seconds 2
if (!(Get-Process explorer -ErrorAction SilentlyContinue)) {
    Start-Process explorer
}
