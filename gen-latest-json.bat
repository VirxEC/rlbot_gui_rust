@echo off
setlocal enabledelayedexpansion

echo %1
echo %2
echo %3

set /p sig=<src-tauri\target\release\bundle\msi\RLBotGUI_%2_x64_en-US.msi.zip.sig
echo %sig%

(
echo { 
echo   "name": "%1",
echo   "notes": %3,
echo   "platforms": {
echo     "windows-x86_64": {
echo       "signature": "%sig%",
echo       "url":"https://github.com/VirxEC/rlbot_gui_rust/releases/download/%1/RLBotGUI_!%2_x64_en-US.msi.zip"
echo     }
echo   }
echo }
)> ..\latest.json