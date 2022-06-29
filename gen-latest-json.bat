setlocal enabledelayedexpansion

dir

echo %1
echo %2

set var1=%1
set var1=!var1:~1!
echo !var1!

set /p sig=<target\release\bundle\msi\RLBotGUI_%var1%_x64_en-US.msi.zip.sig
echo %sig%

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
)> ../latest.json