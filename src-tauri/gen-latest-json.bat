@echo off
setlocal enabledelayedexpansion

rem Version, e.x. v1.0.0
echo %1
rem Release notes, e.x. "my release notes"
echo %2
rem File signature, e.x. "w97etgho4gbv="
echo %3

set var1=%1
set var1=!var1:~1!
echo !var1!

(
echo { 
echo   "name": "%1",
echo   "notes": %2,
echo   "platforms": {
echo     "windows-x86_64": {
echo       "signature": %3,
echo       "url":"https://github.com/VirxEC/rlbot_gui_rust/releases/download/%1/RLBotGUI_!%var1%_x64_en-US.msi.zip"
echo     }
echo   }
echo }
)> latest.json