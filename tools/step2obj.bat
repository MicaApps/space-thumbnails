@echo off
setlocal EnableDelayedExpansion
REM chcp 65001 >nul

REM Generate a unique log file for this execution
if defined STEP2OBJ_OUTPUT (
    set "LOG_FILE=%STEP2OBJ_OUTPUT%.log"
) else (
    set "LOG_FILE=%TEMP%\space_thumbnails_bat_%RANDOM%.log"
)

set SCRIPT_DIR=%~dp0
echo [step2obj] Starting in %SCRIPT_DIR% 
echo [step2obj] Input: %STEP2OBJ_INPUT% 
echo [step2obj] Output: %STEP2OBJ_OUTPUT% 
echo [step2obj] Date: %DATE% %TIME% 

REM Check for python command
where python 
python --version 

REM Use local Embedded Python (Plan A)
if exist "%SCRIPT_DIR%python\python.exe" (
    echo [step2obj] Using local python 
    set PYTHONIOENCODING=utf-8
    "%SCRIPT_DIR%python\python.exe" -u "%SCRIPT_DIR%step2obj_occ.py" %* 
    set EXIT_CODE=!ERRORLEVEL!
    goto :END
)

REM Use System Python (Plan B - Explicit Path)
if exist "C:\Users\Shomn\AppData\Local\Programs\Python\Python311\python.exe" (
    echo [step2obj] Using explicit python path 
    set PYTHONIOENCODING=utf-8
    "C:\Users\Shomn\AppData\Local\Programs\Python\Python311\python.exe" -u "%SCRIPT_DIR%step2obj_occ.py" %* 
    set EXIT_CODE=!ERRORLEVEL!
    goto :END
)

REM Use System Python (Plan C - PATH)
where python >nul 2>nul
if !ERRORLEVEL! equ 0 (
    echo [step2obj] Using system python 
    set PYTHONIOENCODING=utf-8
    python -u "%SCRIPT_DIR%step2obj_occ.py" %* 
    set EXIT_CODE=!ERRORLEVEL!
    goto :END
)

echo [step2obj.bat] Error: No Python environment found! >> "!LOG_FILE!"
set EXIT_CODE=1

:END
echo [step2obj] Exit Code: !EXIT_CODE! >> "%LOG_FILE%"
exit /b !EXIT_CODE!
