@echo off
setlocal enabledelayedexpansion

set VERSION=0.1.0

echo Building RTML v%VERSION% for Windows...
cargo build --release
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

set OUTFILE=rtml-%VERSION%-x86_64-windows.exe
copy target\release\rtml.exe %OUTFILE% >nul
echo Done: %CD%\%OUTFILE%
