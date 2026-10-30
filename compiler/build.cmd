@echo off
rem Builds the compiler tools with MSVC (Visual Studio 2022) into compiler\bin.
rem   cc0, cpp, copt : shared by every target
rem   cc1.hbc2, cc2.hbc2 : the HBC-2 parser target and backend
rem   cc1.ref, cc2.ref : Fuzix's reference parser target and default backend
setlocal
set VCVARS=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat
call "%VCVARS%" >nul 2>nul || (echo cannot run "%VCVARS%" & exit /b 1)
set K=%~dp0fcc
set C=%~dp0msvc
set O=%~dp0bin
set B=%~dp0build
set FLAGS=/nologo /std:c11 /W3 /O2 /D_CRT_SECURE_NO_WARNINGS /D_CRT_NONSTDC_NO_WARNINGS /I"%C%" /FI"%C%\compat.h" /DLIBPATH="\"lib\"" /DBINPATH="\"bin\""
set CC1=body.c declaration.c enum.c error.c expression.c header.c idxdata.c initializer.c label.c lex.c main.c primary.c stackframe.c storage.c struct.c switch.c symbol.c tree.c type.c type_iterator.c volatile.c
if not exist "%O%" mkdir "%O%"
if not exist "%B%" mkdir "%B%"
call :tool cc0 frontend.c || exit /b 1
call :tool cpp Preprocessor/cpp.c Preprocessor/hash.c Preprocessor/main.c Preprocessor/token1.c Preprocessor/token2.c || exit /b 1
call :tool copt copt.c || exit /b 1
call :tool cc1.ref %CC1% target-6800.c || exit /b 1
call :tool cc2.ref backend.c backend-default.c || exit /b 1
call :tool cc1.hbc2 %CC1% target-hbc2.c || exit /b 1
call :tool cc2.hbc2 backend.c backend-hbc2.c || exit /b 1
echo build OK
exit /b 0

:tool
set NAME=%1
shift
set SRCS=
:args
if "%~1"=="" goto compile
set SRCS=%SRCS% "%K%\%~1"
shift
goto args
:compile
if not exist "%B%\%NAME%" mkdir "%B%\%NAME%"
cl %FLAGS% /Fo"%B%\%NAME%\\" /Fe"%O%\%NAME%.exe" %SRCS% "%C%\compat.c" > "%B%\%NAME%.log" 2>&1
if errorlevel 1 (echo FAILED %NAME%, see build\%NAME%.log & findstr /i /c:" error " "%B%\%NAME%.log" & exit /b 1)
echo built %NAME%
exit /b 0
