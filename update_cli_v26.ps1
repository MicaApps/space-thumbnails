$ErrorActionPreference = "Stop"

$targetDir = "target_temp_debug_v26\release"
if (!(Test-Path $targetDir)) {
    New-Item -ItemType Directory -Path $targetDir -Force
}

Write-Host "Building CLI (release)..."
cargo build --release --bin space-thumbnails-cli

Write-Host "Copying CLI to $targetDir..."
Copy-Item "target\release\space-thumbnails-cli.exe" -Destination "$targetDir\space-thumbnails-cli.exe" -Force
# Copy-Item "target\release\space-thumbnails-cli.pdb" -Destination "$targetDir\space-thumbnails-cli.pdb" -Force

Write-Host "Copying tools directory to $targetDir..."
if (Test-Path "$targetDir\tools") {
    Remove-Item "$targetDir\tools" -Recurse -Force
}
Copy-Item "tools" -Destination "$targetDir" -Recurse -Force

Write-Host "Done."
