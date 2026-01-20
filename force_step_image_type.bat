@echo off
echo ==========================================
echo      SpaceThumbnails - Force Image Identity
echo ==========================================

echo 1. Killing Explorer...
taskkill /f /im explorer.exe

echo.
echo 2. Forcing .step to identify as an IMAGE type...
:: This is the magic key that tells Windows "Treat this like a JPG/PNG"
reg add "HKEY_CLASSES_ROOT\.step" /v "PerceivedType" /t REG_SZ /d "image" /f
reg add "HKEY_CLASSES_ROOT\.step" /v "Content Type" /t REG_SZ /d "image/x-step" /f

echo.
echo 3. Also forcing identity in SystemFileAssociations...
reg add "HKEY_LOCAL_MACHINE\SOFTWARE\Classes\SystemFileAssociations\.step" /v "PerceivedType" /t REG_SZ /d "image" /f
reg add "HKEY_LOCAL_MACHINE\SOFTWARE\Classes\SystemFileAssociations\.step" /v "Content Type" /t REG_SZ /d "image/x-step" /f

echo.
echo 4. Restarting Explorer...
start explorer.exe

echo.
echo ==========================================
echo Identity Forced. Please check thumbnails.
echo ==========================================
pause
