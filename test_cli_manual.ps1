
$cliPath = "target_temp_debug_v24\release\space-thumbnails-cli.exe"
$stepFile = "assets\test.step"
$outputFile = "assets\test_cli_output.png"

if (Test-Path $outputFile) { Remove-Item $outputFile }

Write-Host "Running CLI..."
& $cliPath --input $stepFile $outputFile --width 256 --height 256 --api default

if ($?) {
    Write-Host "CLI run successful."
    if (Test-Path $outputFile) {
        Write-Host "Output file created: $outputFile"
    } else {
        Write-Host "Error: Output file not created."
    }
} else {
    Write-Host "CLI run failed."
}
