@echo off
echo Checking Environment...
set "PYTHON=C:\Users\Shomn\AppData\Local\Programs\Python\Python311\python.exe"
if exist "%PYTHON%" (
    echo Python found at: %PYTHON%
    "%PYTHON%" --version
) else (
    echo Python NOT found at: %PYTHON%
)

set "SCRIPT=D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\tools\step2obj_occ.py"
if exist "%SCRIPT%" (
    echo Script found at: %SCRIPT%
) else (
    echo Script NOT found at: %SCRIPT%
)
pause
