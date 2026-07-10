@echo off
setlocal

cd /d "%~dp0"

echo Starte Autorenprogramm...
npm.cmd run tauri dev

if errorlevel 1 (
  echo.
  echo Der Start ist fehlgeschlagen.
  echo Pruefe, ob Node.js installiert ist und die Abhaengigkeiten mit npm install installiert wurden.
  echo.
  pause
)
