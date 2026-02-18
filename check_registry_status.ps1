
$extensions = @(".step", ".stp")
$clsidHandler = "{662657D4-0325-4632-9154-116584281360}"
$clsidHandlerStp = "{552657D4-0325-4632-9154-116584281359}"
$iidThumbnail = "{E357FCCD-A995-4576-B01F-234630154E96}"

function Get-RegistryValue {
    param($path, $name)
    try {
        $val = Get-ItemProperty -Path $path -Name $name -ErrorAction SilentlyContinue
        if ($val) { return $val.$name }
        $val = Get-ItemProperty -Path $path -Name "(default)" -ErrorAction SilentlyContinue # Default value
        if ($val) { return $val."(default)" }
        return "N/A"
    } catch {
        return "Not Found"
    }
}

Write-Host "Checking Registry Configuration..."

foreach ($ext in $extensions) {
    Write-Host "`nExtension: $ext"
    
    # Check HKCR (User + System merged view)
    $hkcrPath = "Registry::HKEY_CLASSES_ROOT\$ext\ShellEx\$iidThumbnail"
    $val = (Get-Item -Path $hkcrPath -ErrorAction SilentlyContinue).GetValue("")
    Write-Host "HKCR\$ext\ShellEx\${iidThumbnail}: $val"

    # Check HKCU
    $hkcuPath = "Registry::HKEY_CURRENT_USER\Software\Classes\$ext\ShellEx\$iidThumbnail"
    $val = (Get-Item -Path $hkcuPath -ErrorAction SilentlyContinue).GetValue("")
    Write-Host "HKCU\Software\Classes\$ext\ShellEx\${iidThumbnail}: $val"

    # Check HKLM
    $hklmPath = "Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Classes\$ext\ShellEx\$iidThumbnail"
    $val = (Get-Item -Path $hklmPath -ErrorAction SilentlyContinue).GetValue("")
    Write-Host "HKLM\SOFTWARE\Classes\$ext\ShellEx\${iidThumbnail}: $val"

    # Check ProgID
    $progId = (Get-Item -Path "Registry::HKEY_CLASSES_ROOT\$ext" -ErrorAction SilentlyContinue).GetValue("")
    Write-Host "HKCR\$ext (ProgID): $progId"
    
    if ($progId) {
        $progIdShellEx = "Registry::HKEY_CLASSES_ROOT\$progId\ShellEx\$iidThumbnail"
        $val = (Get-Item -Path $progIdShellEx -ErrorAction SilentlyContinue).GetValue("")
        Write-Host "HKCR\$progId\ShellEx\${iidThumbnail}: $val"
    }
}

Write-Host "`nChecking CLSID Registration..."
$clsids = @($clsidHandler, $clsidHandlerStp)

foreach ($clsid in $clsids) {
    Write-Host "`nCLSID: $clsid"
    
    # InProcServer32
    $hkcrInproc = "Registry::HKEY_CLASSES_ROOT\CLSID\$clsid\InProcServer32"
    $dll = (Get-Item -Path $hkcrInproc -ErrorAction SilentlyContinue).GetValue("")
    $model = (Get-Item -Path $hkcrInproc -ErrorAction SilentlyContinue).GetValue("ThreadingModel")
    Write-Host "HKCR InProcServer32 DLL: $dll"
    Write-Host "HKCR ThreadingModel: $model"

    $hkcuInproc = "Registry::HKEY_CURRENT_USER\Software\Classes\CLSID\$clsid\InProcServer32"
    $dll = (Get-Item -Path $hkcuInproc -ErrorAction SilentlyContinue).GetValue("")
    Write-Host "HKCU InProcServer32 DLL: $dll"
}
