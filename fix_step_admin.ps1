
$ErrorActionPreference = "Continue"

# Check for Admin rights
if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")) {
    Write-Warning "This script requires Administrator privileges!"
    exit
}

$dll_path = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\target\release\space_thumbnails_windows.dll"
$step_clsid = "{662657d4-0325-4632-9154-116584281360}"
$stp_clsid = "{552657d4-0325-4632-9154-116584281359}"
$shellex_guid = "{E357FCCD-A995-4576-B01F-234630154E96}"

# Helper function for Registry (HKLM default for Admin)
function Set-HKLM {
    param($path, $name, $value, $type="String")
    if (!(Test-Path $path)) {
        New-Item -Path $path -Force | Out-Null
    }
    Set-ItemProperty -Path $path -Name $name -Value $value -Type $type
    Write-Host "Set HKLM: $path ($name) = $value"
}

Write-Host "--- Fixing STEP Association in HKLM (Admin) ---"

# 1. Register CLSIDs in HKLM
Set-HKLM "HKLM:\SOFTWARE\Classes\CLSID\$step_clsid" "(default)" "SpaceThumbnails STEP Provider"
Set-HKLM "HKLM:\SOFTWARE\Classes\CLSID\$step_clsid\InProcServer32" "(default)" $dll_path
Set-HKLM "HKLM:\SOFTWARE\Classes\CLSID\$step_clsid\InProcServer32" "ThreadingModel" "Apartment"

Set-HKLM "HKLM:\SOFTWARE\Classes\CLSID\$stp_clsid" "(default)" "SpaceThumbnails STP Provider"
Set-HKLM "HKLM:\SOFTWARE\Classes\CLSID\$stp_clsid\InProcServer32" "(default)" $dll_path
Set-HKLM "HKLM:\SOFTWARE\Classes\CLSID\$stp_clsid\InProcServer32" "ThreadingModel" "Apartment"

# 2. Create/Update ProgID in HKLM
Set-HKLM "HKLM:\SOFTWARE\Classes\SpaceThumbnails.StepFile" "(default)" "STEP 3D Model"
Set-HKLM "HKLM:\SOFTWARE\Classes\SpaceThumbnails.StepFile" "FriendlyTypeName" "STEP 3D Model"
Set-HKLM "HKLM:\SOFTWARE\Classes\SpaceThumbnails.StepFile" "AlwaysShowExt" ""
Set-HKLM "HKLM:\SOFTWARE\Classes\SpaceThumbnails.StepFile\DefaultIcon" "(default)" "%SystemRoot%\System32\imageres.dll,2"
Set-HKLM "HKLM:\SOFTWARE\Classes\SpaceThumbnails.StepFile\Shell\Open\Command" "(default)" "notepad.exe `"%1`""
Set-HKLM "HKLM:\SOFTWARE\Classes\SpaceThumbnails.StepFile\ShellEx\$shellex_guid" "(default)" $step_clsid

# 3. Associate Extensions in HKLM
# .step
Set-HKLM "HKLM:\SOFTWARE\Classes\.step" "(default)" "SpaceThumbnails.StepFile"
Set-HKLM "HKLM:\SOFTWARE\Classes\.step" "PerceivedType" "Document"
Set-HKLM "HKLM:\SOFTWARE\Classes\.step" "Content Type" "model/step"
Set-HKLM "HKLM:\SOFTWARE\Classes\.step\ShellEx\$shellex_guid" "(default)" $step_clsid

# .stp (using same ProgID but different CLSID for thumbnail if needed, logic says stp_clsid)
Set-HKLM "HKLM:\SOFTWARE\Classes\.stp" "(default)" "SpaceThumbnails.StepFile"
Set-HKLM "HKLM:\SOFTWARE\Classes\.stp" "PerceivedType" "Document"
Set-HKLM "HKLM:\SOFTWARE\Classes\.stp" "Content Type" "model/step"
Set-HKLM "HKLM:\SOFTWARE\Classes\.stp\ShellEx\$shellex_guid" "(default)" $stp_clsid

# 4. SystemFileAssociations in HKLM (Backup)
Set-HKLM "HKLM:\SOFTWARE\Classes\SystemFileAssociations\.step\ShellEx\$shellex_guid" "(default)" $step_clsid
Set-HKLM "HKLM:\SOFTWARE\Classes\SystemFileAssociations\.stp\ShellEx\$shellex_guid" "(default)" $stp_clsid

# 5. Add to Approved List (Just in case)
Set-HKLM "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Shell Extensions\Approved" "$step_clsid" "SpaceThumbnails STEP"
Set-HKLM "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Shell Extensions\Approved" "$stp_clsid" "SpaceThumbnails STP"

# 6. Force Refresh via cmd (assoc/ftype)
cmd /c assoc .step=SpaceThumbnails.StepFile
cmd /c assoc .stp=SpaceThumbnails.StepFile
cmd /c ftype SpaceThumbnails.StepFile=notepad.exe "%1"

Write-Host "Restarting Explorer..."
Stop-Process -Name explorer -Force
Start-Sleep -Seconds 2
if (!(Get-Process explorer -ErrorAction SilentlyContinue)) {
    Start-Process explorer
}
