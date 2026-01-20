@echo off
echo ==========================================
echo      SpaceThumbnails Deep Clean Fix
echo ==========================================

echo 1. Killing Explorer to unlock registry files...
taskkill /f /im explorer.exe

echo.
echo 2. Deleting UserChoice Lock (The most likely culprit)...
reg delete "HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.step\UserChoice" /f

echo.
echo 3. Deleting User-Level Class Overrides...
reg delete "HKEY_CURRENT_USER\Software\Classes\.step" /f

echo.
echo 4. Cleaning OpenWithList to remove Keyshot priority...
reg delete "HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.step\OpenWithList" /f
reg delete "HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.step\OpenWithProgids" /f

echo.
echo 5. Re-asserting SpaceThumbnails Authority...
:: Re-add Global Handler
reg add "HKEY_CLASSES_ROOT\.step\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}" /ve /t REG_SZ /d "{662657D4-0325-4632-9154-116584281360}" /f
:: Re-add System Handler
reg add "HKEY_LOCAL_MACHINE\SOFTWARE\Classes\SystemFileAssociations\.step\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}" /ve /t REG_SZ /d "{662657D4-0325-4632-9154-116584281360}" /f

echo.
echo 6. Restarting Explorer...
start explorer.exe

echo.
echo ==========================================
echo Fix Complete. Please check if thumbnails appear.
echo ==========================================
pause
