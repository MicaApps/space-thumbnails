$searchStr = "target_temp_debug_v3"

Write-Host "Searching HKCU for '$searchStr'..."
Get-ChildItem -Path "HKCU:\Software\Classes\CLSID" -Recurse -ErrorAction SilentlyContinue | ForEach-Object {
    $path = $_.PSPath
    try {
        $val = (Get-ItemProperty -Path $path -Name "(default)" -ErrorAction SilentlyContinue)."(default)"
        if ($val -like "*$searchStr*") {
            Write-Host "FOUND in HKCU: $path = $val"
        }
    } catch {}
}

Write-Host "Searching HKLM for '$searchStr'..."
Get-ChildItem -Path "HKLM:\SOFTWARE\Classes\CLSID" -Recurse -ErrorAction SilentlyContinue | ForEach-Object {
    $path = $_.PSPath
    try {
        $val = (Get-ItemProperty -Path $path -Name "(default)" -ErrorAction SilentlyContinue)."(default)"
        if ($val -like "*$searchStr*") {
            Write-Host "FOUND in HKLM: $path = $val"
        }
    } catch {}
}

Write-Host "Searching HKCR for '$searchStr'..."
Get-ChildItem -Path "HKCR:\CLSID" -Recurse -ErrorAction SilentlyContinue | ForEach-Object {
    $path = $_.PSPath
    try {
        $val = (Get-ItemProperty -Path $path -Name "(default)" -ErrorAction SilentlyContinue)."(default)"
        if ($val -like "*$searchStr*") {
            Write-Host "FOUND in HKCR: $path = $val"
        }
    } catch {}
}
