@echo off
rem Décompile les mini-jeux de Dokuro-chan avec Ghidra, en pseudo-C, dans
rem decompiled (chaque fichier avec les fonctions
rem qu'elles appellent) :
rem   mini_mogura.c, mini_run.c, mini_card.c, mini_catch.c, mini_bat.c :
rem                les cinq mini-jeux (Mini_Mogura*, Mini_Run*…) ;
rem   ad_mini.c  : le cadre commun des mini-jeux (AD_MiniGame*) ;
rem   font.c     : la police et ses textes (FW_*, MkFont*, RunFontSystem,
rem                InitFontSystem, AD_InformationWin*, AD_CheckWin*) ;
rem   window.c   : les fenêtres et le curseur (AD_Win*, MkWindow*, Cursor_*) ;
rem   stcm2.c    : la machine des scripts et ses instructions (STCM2_*) ;
rem   adv.c      : l'enchaînement du jeu, la fenêtre de texte, les choix
rem                (adv_*, AD_Adv*, AD_Scrirpt*, AD_ModeChange*…) ;
rem   adv_gp.c   : décors, personnages, animations, effets, fondus,
rem                caméra, drapeaux (AD_BG*, AD_TrChara*, AD_Effect*…) ;
rem   menus.c    : titre, menu principal, options, album, historique ;
rem   map.c      : la carte (AD_Map*) ;
rem   battle.c   : le combat (AD_Dcb*) ;
rem   tenkey.c   : le pavé numérique, la répartition des points, les
rem                résultats (AD_Tenkey*, AD_Distribute*, AD_Result*) ;
rem   save.c     : les sauvegardes (AD_*Save*, AD_*Load*, mcsm*) ;
rem   mcs.c      : l'écran de la carte mémoire (mcs*, prc_*, ctlFileNum*,
rem                MemorycardPart, les en-têtes et le curseur des fichiers) ;
rem   sound.c    : musiques, bruitages, voix (AD_BGM*, AD_SE*, Snd_*) ;
rem   movie.c    : les vidéos (Movie*, readMpeg…) ;
rem   extra.c    : les préfixes donnés en second argument, s'il y en a.
rem L'exécutable est d'abord sorti de l'ISO (extract.py), ses noms de
rem fonctions tirés de ses informations de débogage (dwarf1_symbols.py).
rem Ce dossier est ignoré par git : il sert seulement de référence de
rem lecture pour le portage, jamais publié.
rem
rem Usage : Tools\decompile.bat <dossier de Ghidra> [préfixes]
rem   préfixes : d'autres fonctions, séparées par « : » (AD_BlackOut:AD_Effect).
rem Il faut Python, Ghidra 12 avec l'extension ghidra-emotionengine-reloaded
rem (processeur R5900 de la PS2) ; Java est pris dans JAVA_HOME (ou celui
rem configuré pour Ghidra).

setlocal
set "ROOT=%~dp0.."
set "GHIDRA=%~1"
if "%GHIDRA%"=="" (
    echo Usage: Tools\decompile.bat ^<Ghidra folder^> [prefixes]
    exit /b 1
)

python -I "%~dp0extract.py"
if errorlevel 1 exit /b 1

set "WORK=%ROOT%\Tools\work"
python -I "%~dp0dwarf1_symbols.py" "%WORK%\SLPM_661.86" "%WORK%\symbols.txt"
if errorlevel 1 exit /b 1

set "OUT=%ROOT%\decompiled"
if exist "%OUT%" rmdir /s /q "%OUT%"
mkdir "%OUT%"
if not exist "%WORK%\ghidra" mkdir "%WORK%\ghidra"
rem Le lanceur de Ghidra coupe les chemins aux espaces : leurs noms courts.
for %%I in ("%WORK%") do set "WORK=%%~sI"
for %%I in ("%OUT%") do set "OUT=%%~sI"
for %%I in ("%~dp0.") do set "SCRIPTS=%%~sI"

rem Le projet Ghidra est d'abord importé, analysé et nommé s'il n'existe pas
rem encore ; les exports se font ensuite en plusieurs passes (le lanceur de
rem Ghidra refuse une ligne de commande trop longue).
if not exist "%WORK%\ghidra\Dokuro.rep" (
    call "%GHIDRA%\support\analyzeHeadless.bat" "%WORK%\ghidra" Dokuro -import "%WORK%\SLPM_661.86" -processor r5900:LE:32:default -scriptPath "%SCRIPTS%" -postScript ApplySymbols.java "%WORK%\symbols.txt"
)

set EXPORT=-postScript ExportMinigame.java "%OUT%\mini_mogura.c" Mini_Mogura 1
set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\mini_run.c" Mini_Run:MiniRun 1
set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\mini_card.c" Mini_Card 1
set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\mini_catch.c" Mini_Catch 1
set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\mini_bat.c" Mini_Bat 1
set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\ad_mini.c" AD_Mini 1
set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\font.c" FW_:MkFont:RunFontSystem:InitFontSystem:AD_InformationWin:AD_CheckWin 1
set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\window.c" AD_Win:MkWindow:Cursor_ 1
call :export

set EXPORT=-postScript ExportMinigame.java "%OUT%\stcm2.c" STCM2_ 1
set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\adv.c" adv_:AD_Adv:AD_ADInit:AD_ADVSystemInit:AD_GameInit:AD_GameGp:AD_GameWin:AD_Scrirpt:AD_StScrirpt:AD_ReadMode:AD_ReadTxt:AD_AutoTime:AD_ModeChange:AD_SelCsr:AD_Text:AD_DataReadStateGet:AD_SystemGp:AD_EndcheckFlag:AD_TimeBackNum:AD_TimeLimit:AD_TimeOver:AD_StartTime 1
set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\adv_gp.c" AD_BG:AD_Chara:AD_TrChara:AD_TrCh:AD_ChEmotion:AD_ChPt:AD_Anime:AD_AniBack:AD_Gp:AD_GP:AD_MV:AD_PtEff:AD_Effect:AD_BlackOut:AD_Camera:AD_Flag 1
call :export

set EXPORT=-postScript ExportMinigame.java "%OUT%\menus.c" AD_Mainmenu:AD_Title:AD_Option:AD_CgLib:AD_CgBonus:AD_BackLog:AD_Endcheck:AD_SaveBg:AD_SaveVoice 1
set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\map.c" AD_Map 1
set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\battle.c" AD_Dcb 1
set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\tenkey.c" AD_Tenkey:AD_TenKey:AD_Distribute:AD_Result:AD_AD_Result 1
call :export

set EXPORT=-postScript ExportMinigame.java "%OUT%\save.c" AD_AdvSave:AD_AdvLoad:AD_AdvSystem:AD_BasicMc:AD_InitMc:AD_LoadMc:AD_SaveMc:AD_SystemMc:AD_SaveInit:AD_SubLoad:AD_SubSave:AD_Rewind:AD_BRead:mcsm:mcsMain:mcsControl 1
set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\sound.c" AD_BGM:AD_SE:AD_LoopSE:AD_VOICE:AD_Voice:AD_Sound:AD_AllSnd:Snd_ 1
set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\movie.c" Movie:StandbyMovie:readMpeg:mpeg:GetMovieStatus 1
set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\mcs.c" mcs:prc_:ctlFileNum:SetSaveMode:SetDefaultCursor:GetLoadListNumber:GetFileNumSprColor:sprFileNum:MemorycardPart:SetMcSystemInfo 1
if not "%~2"=="" set EXPORT=%EXPORT% -postScript ExportMinigame.java "%OUT%\extra.c" %~2 1
call :export
exit /b 0

rem Une passe d'exports sur le projet déjà analysé.
:export
call "%GHIDRA%\support\analyzeHeadless.bat" "%WORK%\ghidra" Dokuro -process SLPM_661.86 -noanalysis -scriptPath "%SCRIPTS%" %EXPORT%
exit /b