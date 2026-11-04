@echo off
rem Lance le portage en debug avec les variables de test (section
rem « Tester » du README) choisies dans un menu : zones de la souris,
rem vidéos du démarrage, trace, histoire à un endroit, captures, appuis.
rem Lance aussi l'audit de l'histoire et les tests. L'ISO doit être à la
rem racine du dépôt (le debug l'y cherche).

setlocal
cd /d "%~dp0.."
rem La console en UTF-8 (celle du fichier) le temps du menu, puis sa page
rem de code d'origine.
for /f "tokens=2 delims=:" %%a in ('chcp') do set "PAGE=%%a"
chcp 65001 >nul

set "ZONES=non"
set "BOOT=non"
set "TRACE=non"
set "STORY="
set "SNAPSHOT="
set "EVERY="
set "INPUT="

:menu
cls
echo  Dokuro-chan : tester le portage (debug)
echo.
echo   1. Zones de la souris (DOKURO_ZONES) ............ %ZONES%
echo   2. Vidéos du démarrage (DOKURO_BOOT) ............ %BOOT%
echo   3. Trace des points d'entrée (DOKURO_TRACE) ..... %TRACE%
echo   4. Histoire à un endroit (DOKURO_STORY) ......... %STORY%
echo   5. Une capture, puis quitter (DOKURO_SNAPSHOT) .. %SNAPSHOT%
echo   6. Captures régulières (DOKURO_EVERY) ........... %EVERY%
echo   7. Appuis simulés (DOKURO_INPUT) ................ %INPUT%
echo.
echo   J. Jouer    A. Audit de l'histoire    T. Tests    Q. Quitter
echo.
choice /c 1234567JATQ /n /m "Choix : "
goto c%errorlevel%

:c1
if "%ZONES%"=="oui" (set "ZONES=non") else (set "ZONES=oui")
goto menu

:c2
if "%BOOT%"=="oui" (set "BOOT=non") else (set "BOOT=oui")
goto menu

:c3
if "%TRACE%"=="oui" (set "TRACE=non") else (set "TRACE=oui")
goto menu

:c4
echo.
echo  Script et point d'entrée, par exemple 102:sure2 (vide : le titre).
set "STORY="
set /p "STORY=DOKURO_STORY = "
goto menu

:c5
echo.
echo  Image et fichier, par exemple 600:capture.png (vide : aucune).
set "SNAPSHOT="
set /p "SNAPSHOT=DOKURO_SNAPSHOT = "
goto menu

:c6
echo.
echo  Toutes les n images, à partir d'une image : 300@600 (vide : aucune).
set "EVERY="
set /p "EVERY=DOKURO_EVERY = "
goto menu

:c7
echo.
echo  Appuis, par exemple 100:start;300-320:down (vide : aucun).
set "INPUT="
set /p "INPUT=DOKURO_INPUT = "
goto menu

rem J : le jeu, avec les variables choisies.
:c8
set "DOKURO_ZONES="
set "DOKURO_BOOT="
set "DOKURO_TRACE="
if "%ZONES%"=="oui" set "DOKURO_ZONES=1"
if "%BOOT%"=="oui" set "DOKURO_BOOT=1"
if "%TRACE%"=="oui" set "DOKURO_TRACE=1"
set "DOKURO_STORY=%STORY%"
set "DOKURO_SNAPSHOT=%SNAPSHOT%"
set "DOKURO_EVERY=%EVERY%"
set "DOKURO_INPUT=%INPUT%"
cargo run
if errorlevel 1 pause
goto menu

rem A : l'audit, depuis le début ou depuis un endroit.
:c9
echo.
echo  Départ, par exemple 121:sure1 (vide : le début de l'histoire).
set "DOKURO_AUDIT="
set /p "DOKURO_AUDIT=DOKURO_AUDIT = "
cargo test audit -- --ignored --nocapture
pause
goto menu

rem T : les tests.
:c10
cargo test
pause
goto menu

:c11
chcp %PAGE% >nul
exit /b 0
