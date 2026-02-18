$ErrorActionPreference = "Stop"
# Copy to a simple path to avoid encoding issues in test script
$SourceFile = "D:\Users\Shomn\OneDrive - MSFT\Work\同学帮忙\任国维\Keyshot渲染\stp\HLD60-J1-G7.STEP.step"
$SimpleInputFile = "$env:TEMP\speed_test_input.step"
Copy-Item -Path $SourceFile -Destination $SimpleInputFile -Force

$OutputFile = "$env:TEMP\test_speed.obj"
$BatFile = ".\tools\step2obj.bat"

function Run-Test {
    param(
        [string]$Deflection,
        [string]$AngDeflection,
        [string]$Relative
    )
    
    $env:STEP2OBJ_INPUT = $SimpleInputFile
    $env:STEP2OBJ_OUTPUT = $OutputFile
    $env:STEP2OBJ_DEFLECTION = $Deflection
    $env:STEP2OBJ_ANG_DEFLECTION = $AngDeflection
    $env:STEP2OBJ_RELATIVE = $Relative
    
    Write-Host "Running Test: Deflection=$Deflection, Ang=$AngDeflection, Relative=$Relative"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    cmd /c $BatFile
    $sw.Stop()
    Write-Host "Time: $($sw.Elapsed.TotalSeconds)s"
    
    if (Test-Path "$OutputFile.log") {
        # Check for specific timing info in log if available, or just errors
        $logContent = Get-Content "$OutputFile.log" -Raw
        if ($logContent -match "Meshing took ([\d\.]+)s") {
            Write-Host "Internal Meshing Time: $($Matches[1])s"
        }
        if ($logContent -match "Exporting took ([\d\.]+)s") {
             Write-Host "Internal Exporting Time: $($Matches[1])s"
        }
        if ($logContent -match "Error") {
             Write-Host "--- Log Errors ---"
             Write-Host $logContent
             Write-Host "------------------"
        }
        Remove-Item "$OutputFile.log"
    }
    if (Test-Path $OutputFile) { Remove-Item $OutputFile }
    if (Test-Path "$($OutputFile -replace '.obj','.mtl')") { Remove-Item "$($OutputFile -replace '.obj','.mtl')" }
}

# 1. Baseline (Current)
Run-Test "10.0" "0.5" "False"

# 2. Coarse Absolute
Run-Test "50.0" "0.8" "False"

# 3. Relative (0.1 = 10% bounding box)
Run-Test "0.1" "0.8" "True"

# 4. Very Coarse Relative (0.3 = 30%)
Run-Test "0.3" "1.0" "True"

# Cleanup
Remove-Item $SimpleInputFile
