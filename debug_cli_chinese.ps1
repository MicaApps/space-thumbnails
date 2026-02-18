$exe = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\target_temp_debug_v10\release\space-thumbnails-cli.exe"
$inputPath = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\assets\中文测试.step"

# Create a dummy file if it doesn't exist
if (-not (Test-Path $inputPath)) {
    Copy-Item "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\assets\test.step" $inputPath
}

$outputPath = "test_chinese_manual.png"

Write-Host "Running CLI (Chinese Path)..."
# Usage: cli.exe <OUTPUT> --input <INPUT>
$p = Start-Process -FilePath $exe -ArgumentList "`"$outputPath`" --input `"$inputPath`" --width 256 --height 256" -NoNewWindow -PassThru -Wait -RedirectStandardError "cli_chinese_error.txt" -RedirectStandardOutput "cli_chinese_output.txt"

Write-Host "Exit Code: $($p.ExitCode)"

if (Test-Path "cli_chinese_output.txt") {
    Write-Host "STDOUT:"
    Get-Content "cli_chinese_output.txt"
}
if (Test-Path "cli_chinese_error.txt") {
    Write-Host "STDERR:"
    Get-Content "cli_chinese_error.txt"
}

if (Test-Path $outputPath) {
    Write-Host "Success: Output file created."
} else {
    Write-Host "Failure: Output file not created."
}
