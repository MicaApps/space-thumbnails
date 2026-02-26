
$ErrorActionPreference = "Continue"

$step_clsid = "{662657d4-0325-4632-9154-116584281360}"
$stp_clsid = "{552657d4-0325-4632-9154-116584281359}"
$shellex_guid = "{E357FCCD-A995-4576-B01F-234630154E96}"

Write-Host "=== DEBUGGING STEP/STP REGISTRY ==="

function Check-Reg {
    param($path, $name)
    $val = Get-ItemProperty -Path $path -Name $name -ErrorAction SilentlyContinue
    if ($val) {
        Write-Host "OK: $path ($name) = $($val.$name)" -ForegroundColor Green
    } else {
        Write-Host "MISSING: $path ($name)" -ForegroundColor Red
    }
}

# 1. Check HKCU .step
Check-Reg "HKCU:\Software\Classes\.step" "(default)"
Check-Reg "HKCU:\Software\Classes\.step\ShellEx\$shellex_guid" "(default)"

# 2. Check HKCU .stp
Check-Reg "HKCU:\Software\Classes\.stp" "(default)"
Check-Reg "HKCU:\Software\Classes\.stp\ShellEx\$shellex_guid" "(default)"

# 3. Check ProgID SpaceThumbnails.StepFile
Check-Reg "HKCU:\Software\Classes\SpaceThumbnails.StepFile\ShellEx\$shellex_guid" "(default)"

# 4. Check CLSID Registration (HKCR merged view)
Check-Reg "Registry::HKEY_CLASSES_ROOT\CLSID\$step_clsid\InProcServer32" "(default)"
Check-Reg "Registry::HKEY_CLASSES_ROOT\CLSID\$stp_clsid\InProcServer32" "(default)"

# 5. Check UserChoice (The Killer)
$uc_step = Get-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.step\UserChoice" -ErrorAction SilentlyContinue
if ($uc_step) {
    Write-Host "WARNING: UserChoice exists for .step -> $($uc_step.ProgId)" -ForegroundColor Yellow
    Write-Host "   This might override our custom ProgID!"
} else {
    Write-Host "OK: No UserChoice for .step" -ForegroundColor Green
}

$uc_stp = Get-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.stp\UserChoice" -ErrorAction SilentlyContinue
if ($uc_stp) {
    Write-Host "WARNING: UserChoice exists for .stp -> $($uc_stp.ProgId)" -ForegroundColor Yellow
} else {
    Write-Host "OK: No UserChoice for .stp" -ForegroundColor Green
}

# 6. Check SystemFileAssociations
Check-Reg "HKCU:\Software\Classes\SystemFileAssociations\.step\ShellEx\$shellex_guid" "(default)"
Check-Reg "HKCU:\Software\Classes\SystemFileAssociations\.stp\ShellEx\$shellex_guid" "(default)"
