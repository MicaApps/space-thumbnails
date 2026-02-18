@echo off
chcp 65001 > nul
set "PYTHON=C:\Users\Shomn\AppData\Local\Programs\Python\Python311\python.exe"
set "SCRIPT=D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\tools\step2obj_occ.py"

echo Checking Python...
if exist "%PYTHON%" (
    echo Python found at %PYTHON%
    "%PYTHON%" --version
) else (
    echo Python NOT found at %PYTHON%
    exit /b 1
)

echo Checking Script...
if exist "%SCRIPT%" (
    echo Script found at %SCRIPT%
) else (
    echo Script NOT found at %SCRIPT%
    exit /b 1
)

echo Running test...
"%PYTHON%" "%SCRIPT%" --help
if %ERRORLEVEL% NEQ 0 (
    echo Script execution failed with code %ERRORLEVEL%
) else (
    echo Script execution successful (help output)
)
