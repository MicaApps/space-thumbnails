
function Set-RegDefault {
    param (
        [string]$Path,
        [string]$Value
    )
    if (!(Test-Path $Path)) {
        New-Item -Path $Path -Force | Out-Null
    }
    Set-Item -Path $Path -Value $Value
    Write-Host "Set default value for $Path"
}

function Set-RegValue {
    param (
        [string]$Path,
        [string]$Name,
        [string]$Value
    )
    if (!(Test-Path $Path)) {
        New-Item -Path $Path -Force | Out-Null
    }
    New-ItemProperty -Path $Path -Name $Name -Value $Value -PropertyType String -Force | Out-Null
    Write-Host "Set $Name = $Value for $Path"
}# Define CLSIDs
$clsidStep = "{662657D4-0325-4632-9154-116584281360}"
$clsidStp = "{552657D4-0325-4632-9154-116584281359}"

# Use original path, but register in HKLM for system-wide access
$dllPath = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\target_temp_debug_v10\release\space_thumbnails_windows_dll.dll"

# 1. Fix CLSID Registration in HKLM (System-wide)
# Register Step CLSID
$clsidKeyStep = "HKLM:\Software\Classes\CLSID\$clsidStep"
Set-RegDefault -Path $clsidKeyStep -Value "SpaceThumbnails Step Handler"
Set-RegValue -Path $clsidKeyStep -Name "DisableProcessIsolation" -Value 1 -PropertyType DWord

$inprocStep = "$clsidKeyStep\InprocServer32"
Set-RegDefault -Path $inprocStep -Value $dllPath
Set-RegValue -Path $inprocStep -Name "ThreadingModel" -Value "Both"

# Register Stp CLSID
$clsidKeyStp = "HKLM:\Software\Classes\CLSID\$clsidStp"
Set-RegDefault -Path $clsidKeyStp -Value "SpaceThumbnails Stp Handler"
Set-RegValue -Path $clsidKeyStp -Name "DisableProcessIsolation" -Value 1 -PropertyType DWord

$inprocStp = "$clsidKeyStp\InprocServer32"
Set-RegDefault -Path $inprocStp -Value $dllPath
Set-RegValue -Path $inprocStp -Name "ThreadingModel" -Value "Both"

# 2. Fix File Associations in HKLM and HKCU
# HKLM for system defaults
Set-RegDefault -Path "HKLM:\Software\Classes\.step\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}" -Value $clsidStep
Set-RegDefault -Path "HKLM:\Software\Classes\.stp\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}" -Value $clsidStp

# HKCU for user override (just in case)
Set-RegDefault -Path "HKCU:\Software\Classes\.step\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}" -Value $clsidStep
Set-RegDefault -Path "HKCU:\Software\Classes\.stp\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}" -Value $clsidStp

# 3. Fix Applications\keyshot.exe in HKLM
Set-RegDefault -Path "HKLM:\Software\Classes\Applications\keyshot.exe\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}" -Value $clsidStep

Write-Host "Registry fixes applied to HKLM and HKCU."

# 3. Verification
foreach ($clsid in @($clsidStep, $clsidStp)) {
    Write-Host "Verifying CLSID: $clsid"
    $clsidGuid = [Guid]$clsid
    $type = [Type]::GetTypeFromCLSID($clsidGuid)

    if ($type) {
        try {
            $obj = [Activator]::CreateInstance($type)
            Write-Host "SUCCESS: Created COM object instance for $clsid!"
            $obj = $null
            [GC]::Collect()
            [GC]::WaitForPendingFinalizers()
        } catch {
            Write-Host "FAILURE: Could not create instance for $clsid. Error: $_"
        }
    } else {
        Write-Host "FAILURE: Could not get type from CLSID $clsid."
    }
}
