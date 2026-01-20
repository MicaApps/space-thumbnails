@echo off
echo ==========================================
echo      SpaceThumbnails - ProgID Reset
echo ==========================================

echo 1. Killing Explorer...
taskkill /f /im explorer.exe

echo.
echo 2. Creating a Clean ProgID...
reg add "HKEY_CLASSES_ROOT\SpaceThumbnails.StepFile" /f
reg add "HKEY_CLASSES_ROOT\SpaceThumbnails.StepFile" /v "FriendlyTypeName" /t REG_SZ /d "STEP 3D Model" /f

:: Register Thumbnail Provider specifically for this ProgID
reg add "HKEY_CLASSES_ROOT\SpaceThumbnails.StepFile\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}" /ve /t REG_SZ /d "{662657D4-0325-4632-9154-116584281360}" /f

echo.
echo 3. Forcing .step to use our ProgID...
reg add "HKEY_CLASSES_ROOT\.step" /ve /t REG_SZ /d "SpaceThumbnails.StepFile" /f
:: Also set Content Type and PerceivedType on the extension itself
reg add "HKEY_CLASSES_ROOT\.step" /v "Content Type" /t REG_SZ /d "image/x-step" /f
reg add "HKEY_CLASSES_ROOT\.step" /v "PerceivedType" /t REG_SZ /d "image" /f

echo.
echo 4. Cleaning UserChoice again to allow the ProgID switch...
reg delete "HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.step\UserChoice" /f

echo.
echo 5. Restarting Explorer...
start explorer.exe

echo.
echo ==========================================
echo ProgID Reassigned. Please check thumbnails.
echo ==========================================
pause
