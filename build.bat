@echo off
rem Compile le portage en release et prépare l'archive à partager :
rem   target\Dokuro-chan.PS2.Windows.zip
rem   (Game ni Natta yo! Dokuro-chan Kenkou Shindan Daisakusen\dokuro_chan_ps2.exe)
rem L'archive ne contient que l'exécutable : chaque joueur fournit sa propre
rem image disque (voir README.md).
rem Pour Linux : build_linux.bat (WSL) ou build.sh.
rem (messages sans accents : la page de code de la console reste celle
rem de l'utilisateur, chcp 65001 change la police de certaines consoles)

setlocal
cd /d "%~dp0"

cargo build --release
if errorlevel 1 goto :fail

rem L'exécutable dans un dossier au nom du jeu, que l'archive garde : on
rem l'extrait sans éparpiller de fichiers. (Pas de « : » comme dans le
rem titre : Windows les refuse dans un nom de dossier.)
set "ZIP=target\Dokuro-chan.PS2.Windows.zip"
set "DIR=target\package\Game ni Natta yo! Dokuro-chan Kenkou Shindan Daisakusen"
if exist "%ZIP%" del "%ZIP%"
if exist "target\package" rmdir /s /q "target\package"
mkdir "%DIR%"
copy /y "target\release\dokuro_chan_ps2.exe" "%DIR%\" >nul
if errorlevel 1 goto :fail
powershell -NoProfile -Command "Compress-Archive -Path '%DIR%' -DestinationPath '%ZIP%'"
if errorlevel 1 goto :fail
echo %ZIP% est disponible.
exit /b 0

rem En cas d'échec, la fenêtre reste ouverte sur le message d'erreur.
:fail
pause
exit /b 1
