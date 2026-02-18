
@echo off
set "STEP2OBJ_INPUT=%~dp0assets\test.step"
set "STEP2OBJ_OUTPUT=%~dp0test_out.obj"

echo Input: %STEP2OBJ_INPUT%
echo Output: %STEP2OBJ_OUTPUT%

call "%~dp0tools\step2obj.bat"

if exist "%STEP2OBJ_OUTPUT%" (
    echo Success: %STEP2OBJ_OUTPUT% created.
) else (
    echo Failure: %STEP2OBJ_OUTPUT% not found.
    echo Check %TEMP%\space_thumbnails_bat_debug.log
)
