
Write-Host "Checking .step registration..."
$step = Get-ItemProperty "HKCU:\Software\Classes\.step\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}" -ErrorAction SilentlyContinue
if ($step) {
    Write-Host "HKCU .step: $($step.'(default)')"
} else {
    Write-Host "HKCU .step not found"
}

Write-Host "Checking .stp registration..."
$stp = Get-ItemProperty "HKCU:\Software\Classes\.stp\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}" -ErrorAction SilentlyContinue
if ($stp) {
    Write-Host "HKCU .stp: $($stp.'(default)')"
} else {
    Write-Host "HKCU .stp not found"
}

Write-Host "Checking CLSID registration..."
$clsidPath = "HKCU:\Software\Classes\CLSID\{65957386-8178-4C85-8025-F796C42C485A}\InprocServer32"
$clsid = Get-ItemProperty $clsidPath -ErrorAction SilentlyContinue
if ($clsid) {
    Write-Host "CLSID Path: $($clsid.'(default)')"
    if (Test-Path $clsid.'(default)') {
        Write-Host "DLL file exists."
    } else {
        Write-Host "DLL file DOES NOT EXIST!"
    }
} else {
    Write-Host "CLSID not found"
}

Write-Host "Attempting to create COM object..."
$clsidGuid = [Guid]"65957386-8178-4C85-8025-F796C42C485A"
$type = [Type]::GetTypeFromCLSID($clsidGuid)
if ($type) {
    try {
        $obj = [Activator]::CreateInstance($type)
        Write-Host "SUCCESS: Created COM object instance!"
        $obj = $null
        [GC]::Collect()
        [GC]::WaitForPendingFinalizers()
    } catch {
        Write-Host "FAILURE: Could not create instance. Error: $_"
    }
} else {
    Write-Host "FAILURE: Could not get type from CLSID."
}
