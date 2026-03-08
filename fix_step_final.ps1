$ErrorActionPreference = "Continue"

# Define CLSIDs
$step_clsid = "{662657d4-0325-4632-9154-116584281360}"
$stp_clsid = "{552657d4-0325-4632-9154-116584281359}"
$shellex_guid = "{E357FCCD-A995-4576-B01F-234630154E96}"

# DLL Path
$dll_path = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\target\release\space_thumbnails_windows.dll"

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

Write-Host "--- Fixing STEP Association & Re-registering DLL ---"

# 1. Re-register DLL using regsvr32 (Silent)
# This ensures all CLSIDs in the DLL point to the current build, overwriting any old versions.
Write-Host "Registering DLL: $dll_path"
Start-Process -FilePath "regsvr32.exe" -ArgumentList "/s `"$dll_path`"" -Wait

# 2. Fix ProgID details for SpaceThumbnails.StepFile in HKCU
# Add DefaultIcon so it's not a white paper. Using shell32.dll,0 (generic file) or a specific icon if available.
# We'll use a generic document icon for now.
Set-RegKey "HKCU:\Software\Classes\SpaceThumbnails.StepFile\DefaultIcon" "(default)" "%SystemRoot%\System32\imageres.dll,2"

# 3. Add Open command (Notepad as fallback, or try to find KeyShot)
# This fixes "double click does nothing".
$keyshot_path = "C:\Program Files\KeyShot10\bin\keyshot.exe" # Example path, likely wrong but better than nothing if user fixes it later.
# Actually, let's just use Notepad for now so user sees SOMETHING, or leave it empty if user prefers "Open With".
# User said "step没反应" which might mean no thumbnail. 
# Let's check if we can find KeyShot from registry.
$keyshot_cmd = Get-ItemProperty -Path "HKCR:\Applications\keyshot.exe\shell\open\command" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty "(default)"
if ($keyshot_cmd) {
    Write-Host "Found KeyShot command: $keyshot_cmd"
    Set-RegKey "HKCU:\Software\Classes\SpaceThumbnails.StepFile\shell\open\command" "(default)" $keyshot_cmd
} else {
    Write-Host "KeyShot not found in registry. Setting Notepad as fallback."
    Set-RegKey "HKCU:\Software\Classes\SpaceThumbnails.StepFile\shell\open\command" "(default)" "notepad.exe `"%1`""
}

# 4. Ensure .step points to SpaceThumbnails.StepFile in HKCU
Set-RegKey "HKCU:\Software\Classes\.step" "(default)" "SpaceThumbnails.StepFile"
Set-RegKey "HKCU:\Software\Classes\.STEP" "(default)" "SpaceThumbnails.StepFile"

# 5. Ensure SystemFileAssociations also has the thumbnail provider (Double insurance)
Set-RegKey "HKCU:\Software\Classes\SystemFileAssociations\.step\ShellEx\$shellex_guid" "(default)" $step_clsid
Set-RegKey "HKCU:\Software\Classes\SystemFileAssociations\.stp\ShellEx\$shellex_guid" "(default)" $stp_clsid

# 6. Restart Explorer
Write-Host "Restarting Explorer..."
Stop-Process -Name explorer -Force
Start-Sleep -Seconds 2
if (!(Get-Process explorer -ErrorAction SilentlyContinue)) {
    Start-Process explorer
}
