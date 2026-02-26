
$step_clsid = "{662657d4-0325-4632-9154-116584281360}"
Write-Host "Testing COM instantiation for CLSID: $step_clsid"

try {
    $type = [Type]::GetTypeFromCLSID($step_clsid)
    if ($type) {
        Write-Host "CLSID found in registry."
        $obj = [Activator]::CreateInstance($type)
        Write-Host "COM Object instantiated successfully!" -ForegroundColor Green
        Write-Host "Object type: $($obj.GetType().FullName)"
    } else {
        Write-Host "CLSID NOT found in registry lookup." -ForegroundColor Red
    }
} catch {
    Write-Host "Failed to instantiate COM object." -ForegroundColor Red
    Write-Host "Error: $_"
    Write-Host "Details: $($_.Exception.Message)"
}
