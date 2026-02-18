$PYTHON = "C:\Users\Shomn\AppData\Local\Programs\Python\Python311\python.exe"
$SCRIPT = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\tools\step2obj_occ.py"
$INPUT_FILE = "D:\Users\Shomn\OneDrive - MSFT\Work\同学帮忙\任国维\Keyshot渲染\stp\HLD25-D5D2-E2 （新）.STEP"
$OUTPUT_FILE = "C:\Users\Shomn\AppData\Local\Temp\test_manual.obj"
$LOG_FILE_OUT = "C:\Users\Shomn\AppData\Local\Temp\manual_run_out.log"
$LOG_FILE_ERR = "C:\Users\Shomn\AppData\Local\Temp\manual_run_err.log"

Write-Host "PYTHON: $PYTHON"
Write-Host "SCRIPT: $SCRIPT"
Write-Host "INPUT: $INPUT_FILE"
Write-Host "OUTPUT: $OUTPUT_FILE"

if (-not (Test-Path $PYTHON)) {
    Write-Error "Python not found at $PYTHON"
    exit 1
}
if (-not (Test-Path $SCRIPT)) {
    Write-Error "Script not found at $SCRIPT"
    exit 1
}
if (-not (Test-Path -LiteralPath $INPUT_FILE)) {
    Write-Error "Input file not found at $INPUT_FILE"
    exit 1
}

# Run the command
$process = Start-Process -FilePath $PYTHON -ArgumentList """$SCRIPT""", """$INPUT_FILE""", """$OUTPUT_FILE""" -PassThru -NoNewWindow -Wait -RedirectStandardOutput $LOG_FILE_OUT -RedirectStandardError $LOG_FILE_ERR

Write-Host "Exit Code: $($process.ExitCode)"
exit $process.ExitCode
