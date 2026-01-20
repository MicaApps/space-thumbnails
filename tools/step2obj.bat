@echo off
set SCRIPT_DIR=%~dp0

REM 1. Try local portable FreeCAD (Scheme A)
if exist "%SCRIPT_DIR%FreeCAD\bin\FreeCADCmd.exe" (
    "%SCRIPT_DIR%FreeCAD\bin\FreeCADCmd.exe" "%SCRIPT_DIR%step2obj.py"
    exit /b %ERRORLEVEL%
)

REM 2. Try hardcoded system path (fallback for dev)
if exist "C:\Program Files\FreeCAD 1.0\bin\freecadcmd.exe" (
    "C:\Program Files\FreeCAD 1.0\bin\freecadcmd.exe" "%SCRIPT_DIR%step2obj.py"
    exit /b %ERRORLEVEL%
)

echo Error: FreeCAD not found. Please copy FreeCAD to "%SCRIPT_DIR%FreeCAD"
exit /b 1