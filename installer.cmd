@echo off
rem Builds the HBC-2 IDE installer: installer\output\HBC-2-IDE-<version>-setup.exe
rem Needs Inno Setup 6 (winget install JRSoftware.InnoSetup) and the C compiler
rem tools built by compiler\build.cmd.
setlocal
cd /d "%~dp0"

set ISCC=%LOCALAPPDATA%\Programs\Inno Setup 6\ISCC.exe
if not exist "%ISCC%" set ISCC=%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe
if not exist "%ISCC%" set ISCC=%ProgramFiles%\Inno Setup 6\ISCC.exe
if not exist "%ISCC%" (
    echo Inno Setup 6 not found: install it with  winget install JRSoftware.InnoSetup
    goto failed
)
if not exist "compiler\bin\cpp.exe" (
    echo The C compiler is not built: run compiler\build.cmd first.
    goto failed
)

rem the version of the workspace (hbc2\Cargo.toml, [workspace.package])
for /f "tokens=2 delims==" %%v in ('findstr /r /c:"^version *=" hbc2\Cargo.toml') do set VERSION=%%~v
set VERSION=%VERSION: =%
set VERSION=%VERSION:"=%

pushd hbc2
cargo build --release -p hbc2-ide -p hbc2-cli
if errorlevel 1 (popd & echo The build failed, see the errors above. & goto failed)
popd

"%ISCC%" /Q /DAppVersion=%VERSION% installer\hbc2-ide.iss
if errorlevel 1 (echo Inno Setup failed, see the errors above. & goto failed)
echo.
echo Installer: installer\output\HBC-2-IDE-%VERSION%-setup.exe
exit /b 0

:failed
pause
exit /b 1
