@echo off
chcp 65001 > nul
set "PYTHON=C:\Users\Shomn\AppData\Local\Programs\Python\Python311\python.exe"
echo PYTHON path: "%PYTHON%"
set "SCRIPT=D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\tools\step2obj_occ.py"
echo SCRIPT path: "%SCRIPT%"
set "INPUT=D:\Users\Shomn\OneDrive - MSFT\Work\同学帮忙\任国维\Keyshot渲染\stp\HLD25-D5D2-E2 （新）.STEP"
echo INPUT path: "%INPUT%"
set "OUTPUT=C:\Users\Shomn\AppData\Local\Temp\test_manual.obj"
echo OUTPUT path: "%OUTPUT%"

if not exist "%PYTHON%" (
    echo ERROR: Python not found at "%PYTHON%"
    exit /b 1
)

if not exist "%SCRIPT%" (
    echo ERROR: Script not found at "%SCRIPT%"
    exit /b 1
)

echo Running command...
"%PYTHON%" "%SCRIPT%" "%INPUT%" "%OUTPUT%" > "C:\Users\Shomn\AppData\Local\Temp\manual_run.log" 2>&1
echo Exit Code: %ERRORLEVEL%
echo Exit Code: %ERRORLEVEL% >> "C:\Users\Shomn\AppData\Local\Temp\manual_run.log"