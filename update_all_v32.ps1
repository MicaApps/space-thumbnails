$ErrorActionPreference = "Stop"

# Define paths
$ProjectRoot = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6"
$TargetDir = "$ProjectRoot\target_temp_debug_v29\release"

# Ensure target directory exists
if (!(Test-Path $TargetDir)) {
    New-Item -ItemType Directory -Path $TargetDir -Force
}

# 1. Build CLI (Release) - BUILD FIRST TO AVOID KILLING EXPLORER ON FAILURE
Write-Host "Building CLI (Release)..."
Set-Location $ProjectRoot
cargo build --release --bin space-thumbnails-cli
if ($LASTEXITCODE -ne 0) { Write-Error "CLI Build Failed"; exit 1 }

# 2. Build DLL (Release)
Write-Host "Building DLL (Release)..."
cargo build --release --lib -p space-thumbnails-windows
if ($LASTEXITCODE -ne 0) { Write-Error "DLL Build Failed"; exit 1 }

# 3. Stop existing processes
Write-Host "Stopping existing processes..."
Stop-Process -Name space-thumbnails-cli -Force -ErrorAction SilentlyContinue
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue

# 4. Deploy files
Write-Host "Deploying files to $TargetDir..."
Copy-Item "$ProjectRoot\target\release\space-thumbnails-cli.exe" -Destination "$TargetDir\space-thumbnails-cli.exe" -Force
Copy-Item "$ProjectRoot\target\release\space_thumbnails_windows.dll" -Destination "$TargetDir\space_thumbnails_windows.dll" -Force

# 4.1 Deploy Tools (Crucial for STEP conversion)
$ToolsTargetDir = "$TargetDir\tools"
if (!(Test-Path $ToolsTargetDir)) {
    New-Item -ItemType Directory -Path $ToolsTargetDir -Force
}

Write-Host "Deploying tools to $ToolsTargetDir..."
Copy-Item "$ProjectRoot\tools\step2obj.bat" -Destination "$ToolsTargetDir\step2obj.bat" -Force
Copy-Item "$ProjectRoot\tools\step2obj_occ.py" -Destination "$ToolsTargetDir\step2obj_occ.py" -Force

# Copy Python environment if it doesn't exist or update it
# This might be slow, so only copy if missing or forced
if (!(Test-Path "$ToolsTargetDir\python")) {
    Write-Host "Copying embedded Python environment (this may take a while)..."
    Copy-Item "$ProjectRoot\tools\python" -Destination "$ToolsTargetDir" -Recurse -Force
} else {
    Write-Host "Python environment already exists in target. Skipping copy to save time."
}

# 5. Copy dependencies (if needed, but they should be there from previous runs)
# Loading.png check
if (!(Test-Path "$TargetDir\Loading.png")) {
    if (Test-Path "$ProjectRoot\target\release\Loading.png") {
        Copy-Item "$ProjectRoot\target\release\Loading.png" -Destination "$TargetDir\Loading.png" -Force
    }
}

# 5.1 Register DLL (New Step)
Write-Host "Registering DLL..."
Start-Process "regsvr32.exe" -ArgumentList "/s `"$TargetDir\space_thumbnails_windows.dll`"" -Wait

# 6. Restart Explorer (using the new DLL)
Write-Host "Restarting Explorer..."
Start-Process explorer

Write-Host "Update Complete (v32 - Optimized STEP Conversion)."
