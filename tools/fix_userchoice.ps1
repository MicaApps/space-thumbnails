
function Set-RegDefault {
    param (
        [string]$Path,
        [string]$Value
    )
    if (!(Test-Path $Path)) {
        New-Item -Path $Path -Force | Out-Null
    }
    Set-Item -Path $Path -Value $Value
    Write-Host "Set default value for $Path to $Value"
}

$clsidStep = "{662657D4-0325-4632-9154-116584281360}"
$clsidStp = "{552657D4-0325-4632-9154-116584281359}"

$extensions = @{
    ".step" = $clsidStep
    ".stp" = $clsidStp
}

# 1. Remove PerceivedType (might conflict with default image handler)
foreach ($ext in $extensions.Keys) {
    $extKey = "HKCU:\Software\Classes\$ext"
    if (Test-Path $extKey) {
        Remove-ItemProperty -Path $extKey -Name "PerceivedType" -ErrorAction SilentlyContinue
        Write-Host "Removed PerceivedType from $extKey (if existed)"
    }
}

# 2. Register in SystemFileAssociations (fallback mechanism)
foreach ($ext in $extensions.Keys) {
    $clsid = $extensions[$ext]
    # HKCU\Software\Classes\SystemFileAssociations\.step\ShellEx\{e357...}
    $sysAssocKey = "HKCU:\Software\Classes\SystemFileAssociations\$ext\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}"
    Set-RegDefault -Path $sysAssocKey -Value $clsid
    Write-Host "Registered SystemFileAssociation for $ext"

    # 3. Register on known ProgIDs (STEP ISO 10303)
    # This is the default value in HKCR\.step, so it might be used if UserChoice is bypassed or matches.
    $progIds = @("STEP ISO 10303", "step_auto_file", "SpaceThumbnails.StepFile")
    foreach ($progName in $progIds) {
        $pidKey = "HKCU:\Software\Classes\$progName\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}"
        Set-RegDefault -Path $pidKey -Value $clsid
        Write-Host "Registered ShellEx for ProgID $progName ($ext)"
    }
}
