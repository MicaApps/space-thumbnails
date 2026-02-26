
$extensions = @(".step", ".glb")

foreach ($ext in $extensions) {
    Write-Host "`n=== Checking $ext ===" -ForegroundColor Cyan
    
    # 1. Check HKCR Extension default value (ProgID)
    $hkcr_progid = (Get-ItemProperty "Registry::HKEY_CLASSES_ROOT\$ext" -Name "(default)" -ErrorAction SilentlyContinue)."(default)"
    Write-Host "HKCR\$ext (Default ProgID): $hkcr_progid"

    # 2. Check UserChoice
    $user_choice = (Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\$ext\UserChoice" -Name "ProgId" -ErrorAction SilentlyContinue).ProgId
    Write-Host "UserChoice ProgID: $user_choice"

    # Determine effective ProgID
    $effective_progid = if ($user_choice) { $user_choice } else { $hkcr_progid }
    Write-Host "Effective ProgID: $effective_progid" -ForegroundColor Yellow

    if ($effective_progid) {
        # 3. Check ShellEx for Effective ProgID
        $shellex_path = "Registry::HKEY_CLASSES_ROOT\$effective_progid\ShellEx\{E357FCCD-A995-4576-B01F-234630154E96}"
        $shellex_val = (Get-ItemProperty $shellex_path -Name "(default)" -ErrorAction SilentlyContinue)."(default)"
        Write-Host "ShellEx for ${effective_progid}: $shellex_val"
        
        if (-not $shellex_val) {
            Write-Host "WARNING: No ShellEx found for effective ProgID!" -ForegroundColor Red
        }
    }

    # 4. Check SystemFileAssociations
    $sfa_path = "Registry::HKEY_CLASSES_ROOT\SystemFileAssociations\$ext\ShellEx\{E357FCCD-A995-4576-B01F-234630154E96}"
    $sfa_val = (Get-ItemProperty $sfa_path -Name "(default)" -ErrorAction SilentlyContinue)."(default)"
    Write-Host "SystemFileAssociations ShellEx: $sfa_val"
}
