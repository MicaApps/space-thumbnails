$ErrorActionPreference = "Stop"

# Use the script's own directory as the base path
$ScriptDir = $PSScriptRoot

# Find all update_all_v*.ps1 files in the same directory as this script
$scripts = Get-ChildItem -Path $ScriptDir -Filter "update_all_v*.ps1"

if (-not $scripts) {
    Write-Error "No update_all_v*.ps1 scripts found in $ScriptDir."
    exit 1
}

# Parse versions and find the latest one
$latest = $scripts | ForEach-Object {
    if ($_.Name -match "update_all_v(\d+)\.ps1") {
        [pscustomobject]@{
            File = $_
            Version = [int]$matches[1]
        }
    }
} | Sort-Object Version -Descending | Select-Object -First 1

if ($latest) {
    Write-Host "Found latest update script: $($latest.File.Name) (Version $($latest.Version))"
    # Execute the latest script
    & $latest.File.FullName
} else {
    Write-Error "Could not parse versions from update scripts in $ScriptDir."
    exit 1
}
