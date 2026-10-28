@echo off
rem Builds the HBC-2 IDE (debug) with the latest code, then opens it.
cd /d "%~dp0hbc2"
cargo build -p hbc2-ide
if errorlevel 1 (
    echo.
    echo The build failed, see the errors above.
    pause
    exit /b 1
)
start "" "target\debug\hbc2-ide.exe" %*
