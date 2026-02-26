
$ErrorActionPreference = "Continue"

function Set-Reg {
    param($path, $name, $value, $type="String")
    if (!(Test-Path $path)) {
        New-Item -Path $path -Force | Out-Null
    }
    Set-ItemProperty -Path $path -Name $name -Value $value -Type $type
    Write-Host "Set $path ($name) = $value"
}

# 1. Enhance .step definition
Set-Reg "HKCU:\Software\Classes\.step" "PerceivedType" "Document"
Set-Reg "HKCU:\Software\Classes\.step" "Content Type" "model/step"

# 2. Enhance .stp definition
Set-Reg "HKCU:\Software\Classes\.stp" "PerceivedType" "Document"
Set-Reg "HKCU:\Software\Classes\.stp" "Content Type" "model/step"

# 3. Enhance SpaceThumbnails.StepFile ProgID
Set-Reg "HKCU:\Software\Classes\SpaceThumbnails.StepFile" "FriendlyTypeName" "STEP 3D Model"
Set-Reg "HKCU:\Software\Classes\SpaceThumbnails.StepFile" "AlwaysShowExt" ""

# 4. Add to OpenWithProgids to ensure visibility
Set-Reg "HKCU:\Software\Classes\.step\OpenWithProgids" "SpaceThumbnails.StepFile" "" "None"
Set-Reg "HKCU:\Software\Classes\.stp\OpenWithProgids" "SpaceThumbnails.StepFile" "" "None"

# 5. Restart Explorer
Write-Host "Restarting Explorer..."
Stop-Process -Name explorer -Force
Start-Sleep -Seconds 2
if (!(Get-Process explorer -ErrorAction SilentlyContinue)) {
    Start-Process explorer
}
