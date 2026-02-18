
$clsid = New-Object Guid "{65957386-8178-4C85-8025-F796C42C485A}"
$type = [System.Type]::GetTypeFromCLSID($clsid)
if ($type -eq $null) {
    Write-Host "Type not found for CLSID"
    exit
}
try {
    $obj = [System.Activator]::CreateInstance($type)
    Write-Host "Successfully created instance of COM object"
    $obj = $null
    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
} catch {
    Write-Host "Failed to create instance: $_"
}
