
$step_clsid = "{662657d4-0325-4632-9154-116584281360}"
$glb_clsid = "{99FF43F0-D914-4A7A-8325-A8013995C41D}"

function Test-COM ($clsid, $name) {
    Write-Host "Testing $name ($clsid)..." -NoNewline
    try {
        $type = [Type]::GetTypeFromCLSID($clsid)
        $obj = [Activator]::CreateInstance($type)
        Write-Host " SUCCESS!" -ForegroundColor Green
        
        # Optional: Check if it implements IThumbnailProvider (hard to do in PS, but instantiation is 90% of the battle)
        if ($obj) {
            [System.Runtime.InteropServices.Marshal]::ReleaseComObject($obj) | Out-Null
        }
    } catch {
        Write-Host " FAILED!" -ForegroundColor Red
        Write-Host $_.Exception.Message -ForegroundColor Yellow
        Write-Host $_.Exception.InnerException.Message -ForegroundColor Gray
    }
}

Test-COM $step_clsid "STEP Thumbnail Provider"
Test-COM $glb_clsid "GLB Thumbnail Provider"
