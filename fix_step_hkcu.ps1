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

# 1. SystemFileAssociations in HKCU (Highest priority fallback)
Set-RegKey "HKCU:\Software\Classes\SystemFileAssociations\.step\ShellEx\$shellex_guid" "(default)" $step_clsid
Set-RegKey "HKCU:\Software\Classes\SystemFileAssociations\.stp\ShellEx\$shellex_guid" "(default)" $stp_clsid

# 2. Ensure .stp has a default ProgID in HKCU (since it had none in HKCR)
# This might help Explorer decide how to handle it.
Set-RegKey "HKCU:\Software\Classes\.stp" "(default)" "SpaceThumbnails.StepFile"

# 3. Ensure SpaceThumbnails.StepFile exists in HKCU and points to the correct handler
# Note: SpaceThumbnails.StepFile uses the STEP CLSID (1360)
Set-RegKey "HKCU:\Software\Classes\SpaceThumbnails.StepFile\ShellEx\$shellex_guid" "(default)" $step_clsid

# 4. Force restart Explorer to pick up changes
Write-Host "Restarting Explorer..."
Stop-Process -Name explorer -Force
Start-Sleep -Seconds 2
if (!(Get-Process explorer -ErrorAction SilentlyContinue)) {
    Start-Process explorer
}
