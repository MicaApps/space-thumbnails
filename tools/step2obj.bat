@echo off
set SCRIPT_DIR=%~dp0

REM Use local Embedded Python (Plan B)
if exist "%SCRIPT_DIR%python\python.exe" (
    "%SCRIPT_DIR%python\python.exe" "%SCRIPT_DIR%step2obj_occ.py"
    exit /b %ERRORLEVEL%
)

echo Error: Python environment not found in "%SCRIPT_DIR%python".
echo Please run setup_occ.ps1 to install dependencies.
exit /b 1