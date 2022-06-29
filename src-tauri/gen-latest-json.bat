@echo off
setlocal enabledelayedexpansion

rem Version, e.x. v1.0.0
echo %1
rem Release notes, e.x. "my release notes"
echo %2

rem Version with first character trimmed - so v1.0.0 becomes 1.0.0
set var1=%1
set var1=!var1:~1!
echo !var1!

rem Get the file signature
set /p sig=<target\release\bundle\msi\RLBotGUI_%var1%_x64_en-US.msi.zip.sig
echo %sig%

rem Generate the file
(
echo { 
echo   "name": "%1",
echo   "notes": %2,
echo   "platforms": {
echo     "windows-x86_64": {
echo       "signature": "%sig%",
echo       "url":"https://github.com/VirxEC/rlbot_gui_rust/releases/download/%1/RLBotGUI_!%var1%_x64_en-US.msi.zip"
echo     }
echo   }
echo }
)> latest.json