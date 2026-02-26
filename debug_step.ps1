$ErrorActionPreference = "Stop"

function Test-CLSID {
    param (
        [string]$ClsidStr,
        [string]$Name
    )
    Write-Host "Testing $Name ($ClsidStr)..." -NoNewline
    try {
        $type = [Type]::GetTypeFromCLSID([Guid]::new($ClsidStr))
        if ($null -eq $type) {
            Write-Host " FAILED (Type not found)" -ForegroundColor Red
            return
        }
        $instance = [Activator]::CreateInstance($type)
        if ($null -ne $instance) {
            Write-Host " SUCCESS (Instantiated)" -ForegroundColor Green
            # Try to cast to IInitializeWithFile (just to see if it supports it, though difficult in PS without definition)
            # But instantiation is the main hurdle for "DllGetClassObject not called".
        } else {
            Write-Host " FAILED (CreateInstance returned null)" -ForegroundColor Red
        }
    } catch {
        Write-Host " FAILED ($($_.Exception.Message))" -ForegroundColor Red
    }
}

Write-Host "--- Debugging COM Registration ---"
Test-CLSID "662657d4-0325-4632-9154-116584281360" "STEP (.step)"
Test-CLSID "552657d4-0325-4632-9154-116584281359" "STP (.stp)"
Test-CLSID "99FF43F0-D914-4A7A-8325-A8013995C41D" "GLB (.glb)"
