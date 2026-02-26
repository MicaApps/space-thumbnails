
$step_clsid = "{662657d4-0325-4632-9154-116584281360}"
$glb_clsid = "{99FF43F0-D914-4A7A-8325-A8013995C41D}"
$shellex_guid = "{E357FCCD-A995-4576-B01F-234630154E96}"

# 1. FIX GLB: Add ShellEx to AppX ProgID
$appx_progid = "AppXmgw6pxxs62rbgfp9petmdyb4fx7rnd4k"
$appx_shellex_path = "Registry::HKEY_CLASSES_ROOT\$appx_progid\ShellEx\$shellex_guid"
Write-Host "Setting GLB ShellEx for AppX..."
New-Item -Path $appx_shellex_path -Force -ErrorAction SilentlyContinue | Out-Null
Set-ItemProperty -Path $appx_shellex_path -Name "(default)" -Value $glb_clsid
Write-Host "Done." -ForegroundColor Green

# 2. FIX STEP: Add ShellEx to HKCU Classes for .step directly
# This is a strong override that usually beats ProgIDs in HKCR
$hkcu_step_path = "HKCU:\Software\Classes\.step\ShellEx\$shellex_guid"
Write-Host "Setting STEP ShellEx in HKCU override..."
New-Item -Path $hkcu_step_path -Force -ErrorAction SilentlyContinue | Out-Null
Set-ItemProperty -Path $hkcu_step_path -Name "(default)" -Value $step_clsid
Write-Host "Done." -ForegroundColor Green

# 3. FIX STEP: Add ShellEx to HKCU Classes for Applications\keyshot.exe
# This targets the UserChoice ProgID specifically in the user hive
$hkcu_keyshot_path = "HKCU:\Software\Classes\Applications\keyshot.exe\ShellEx\$shellex_guid"
Write-Host "Setting KeyShot ShellEx in HKCU override..."
New-Item -Path $hkcu_keyshot_path -Force -ErrorAction SilentlyContinue | Out-Null
Set-ItemProperty -Path $hkcu_keyshot_path -Name "(default)" -Value $step_clsid
Write-Host "Done." -ForegroundColor Green

# Restart Explorer to apply
Write-Host "Restarting Explorer..."
Stop-Process -ProcessName explorer -Force
Start-Process explorer
