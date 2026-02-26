
$ErrorActionPreference = "Stop"

$cli_path = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\target\release\space-thumbnails-cli.exe"
$input_file = "D:\Users\Shomn\OneDrive - MSFT\Work\同学帮忙\任国维\Keyshot渲染\stp\HLD25-D5D2-E2 （新）.STEP"
$output_file = "C:\Users\Shomn\AppData\Local\Temp\manual_test_thumb.png"

if (-not (Test-Path $cli_path)) {
    Write-Error "CLI not found at $cli_path"
}

if (-not (Test-Path -LiteralPath $input_file)) {
    Write-Warning "Input file not found. Using a dummy file for testing CLI args only (will fail conversion but test CLI launch)."
    # Create a dummy file just to test CLI startup
    $input_file = "C:\Users\Shomn\AppData\Local\Temp\dummy.step"
    New-Item -Path $input_file -ItemType File -Force | Out-Null
}

Write-Host "Running CLI test..."
Write-Host "CLI: $cli_path"
Write-Host "Input: $input_file"
Write-Host "Output: $output_file"

# The CLI arguments are: <input_file> <output_file> --width <w> --height <h>
$process = Start-Process -FilePath $cli_path -ArgumentList """$input_file""", """$output_file""", "--width", "256", "--height", "256" -PassThru -NoNewWindow -Wait

Write-Host "CLI Exit Code: $($process.ExitCode)"

if ($process.ExitCode -eq 0) {
    Write-Host "CLI executed successfully!" -ForegroundColor Green
    if (Test-Path $output_file) {
        Write-Host "Thumbnail generated at $output_file" -ForegroundColor Green
    } else {
        Write-Error "CLI exited with 0 but output file missing!"
    }
} else {
    Write-Host "CLI failed with exit code $($process.ExitCode)" -ForegroundColor Red
}
