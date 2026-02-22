@echo off
setlocal EnableDelayedExpansion

echo [DEBUG] Space Thumbnails Diagnostic Tool
echo ========================================

set "SCRIPT_DIR=%~dp0"
set "PYTHON_DIR=%SCRIPT_DIR%python"
set "CLI_PATH=%SCRIPT_DIR%..\space-thumbnails-cli.exe"

REM Check VC++ Runtime
echo [DEBUG] Checking for VC++ Runtime DLLs in python dir...
if exist "%PYTHON_DIR%\msvcp140.dll" (
    echo [OK] msvcp140.dll found
) else (
    echo [WARN] msvcp140.dll NOT found
)

if exist "%PYTHON_DIR%\vcruntime140.dll" (
    echo [OK] vcruntime140.dll found
) else (
    echo [WARN] vcruntime140.dll NOT found
)

REM Check Python
echo [DEBUG] Checking Python environment...
if exist "%PYTHON_DIR%\python.exe" (
    "%PYTHON_DIR%\python.exe" --version
    if !ERRORLEVEL! neq 0 (
        echo [FAIL] Python execution failed!
    ) else (
        echo [OK] Python executes correctly.
    )
) else (
    echo [FAIL] python.exe not found!
)

REM Check OCP Import
echo [DEBUG] Checking OCP library import...
"%PYTHON_DIR%\python.exe" -c "import OCP; print('OCP imported successfully')"
if !ERRORLEVEL! neq 0 (
    echo [FAIL] OCP import failed! This usually means VC++ Redistributable is missing or corrupted.
) else (
    echo [OK] OCP imported successfully.
)

REM Check CLI
echo [DEBUG] Checking CLI executable...
if exist "%CLI_PATH%" (
    echo [OK] CLI executable found at %CLI_PATH%
    REM Try running with --help to check for DLL errors
    "%CLI_PATH%" --help >nul 2>&1
    if !ERRORLEVEL! neq 0 (
        echo [FAIL] CLI execution failed! Likely missing DLLs or graphics driver issue.
    ) else (
        echo [OK] CLI runs basic help command.
    )
) else (
    echo [WARN] CLI executable not found in parent directory.
    if exist "%SCRIPT_DIR%space-thumbnails-cli.exe" (
         echo [OK] CLI executable found in current directory.
         set "CLI_PATH=%SCRIPT_DIR%space-thumbnails-cli.exe"
    )
)

echo.
echo [INFO] Diagnostic complete. Log files may be in %TEMP%\space-thumbnails*.log
pause
