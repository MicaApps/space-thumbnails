$clsidStep = "{662657D4-0325-4632-9154-116584281360}"
Write-Host "Checking CLSID: $clsidStep"
$type = [Type]::GetTypeFromCLSID([Guid]$clsidStep)
if ($type) {
    try {
        $obj = [Activator]::CreateInstance($type)
        Write-Host "SUCCESS: Created COM object instance!"
    } catch {
        Write-Host "FAILURE: $_"
    }
} else {
    Write-Host "FAILURE: Type not found"
}
