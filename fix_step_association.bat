@echo off
echo Fixing STEP file associations for SpaceThumbnails...

:: 1. Force Global Association (HKCR)
reg add "HKEY_CLASSES_ROOT\.step\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}" /ve /t REG_SZ /d "{662657D4-0325-4632-9154-116584281360}" /f

:: 2. Force SystemFileAssociations (Higher Priority)
reg add "HKEY_LOCAL_MACHINE\SOFTWARE\Classes\SystemFileAssociations\.step\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}" /ve /t REG_SZ /d "{662657D4-0325-4632-9154-116584281360}" /f

:: 3. Clean User Overrides (HKCU) - Try to remove conflicting handlers
:: Note: We don't delete the whole key to avoid breaking "Open With", just the ShellEx part if it exists
reg delete "HKEY_CURRENT_USER\Software\Classes\.step\ShellEx" /f 2>nul

echo Done. Restarting Explorer...
taskkill /f /im explorer.exe
start explorer.exe
pause
