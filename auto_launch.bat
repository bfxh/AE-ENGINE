@echo off
echo ==========================================
echo   WASTELAND PROJECT - AUTOMATIC LAUNCHER
echo ==========================================
echo.

:: 1. 检查是否有Godot在PATH或已知位置
set GODOT_FOUND=0

where godot >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    echo [1/4] Found Godot in PATH
    set GODOT_EXE=godot
    set GODOT_FOUND=1
)

if %GODOT_FOUND% EQU 0 (
    if exist "C:\Program Files\Godot Engine\Godot_4.6\Godot_v4.6-stable_win64.exe" (
        echo [1/4] Found Godot in Program Files
        set GODOT_EXE="C:\Program Files\Godot Engine\Godot_4.6\Godot_v4.6-stable_win64.exe"
        set GODOT_FOUND=1
    )
)

if %GODOT_FOUND% EQU 0 (
    if exist "D:\Program Files\Godot Engine\Godot_4.6\Godot_v4.6-stable_win64.exe" (
        echo [1/4] Found Godot in D drive
        set GODOT_EXE="D:\Program Files\Godot Engine\Godot_4.6\Godot_v4.6-stable_win64.exe"
        set GODOT_FOUND=1
    )
)

if %GODOT_FOUND% EQU 0 (
    if exist "bin\godot.exe" (
        echo [1/4] Found Godot in local bin folder
        set GODOT_EXE="%~dp0bin\godot.exe"
        set GODOT_FOUND=1
    )
)

if %GODOT_FOUND% EQU 0 (
    echo [ERROR] Godot not found!
    echo Please install Godot 4.6 from:
    echo   https://godotengine.org/download/windows/
    echo And either:
    echo   1. Add Godot to your PATH, or
    echo   2. Place Godot executable in bin\godot.exe
    pause
    exit /b 1
)

:: 2. 检查GDExtension DLL
echo [2/4] Checking GDExtension...
set DLL_PATH="%~dp0godot_project\bin\wasteland_gdextension.dll"

if not exist %DLL_PATH% (
    echo [WARNING] GDExtension DLL not found!
    echo Trying to build...
    cargo build --release -p wasteland_gdextension
    if %ERRORLEVEL% EQU 0 (
        echo [OK] Build complete!
        copy target\release\wasteland_gdextension.dll godot_project\bin\
    )
)

:: 3. 启动游戏
echo [3/4] Starting Wasteland...
echo Using Godot: %GODOT_EXE%
echo.
echo Press any key to start, or Ctrl+C to cancel...
pause >nul

echo [4/4] Launching...
cd "%~dp0godot_project"
%GODOT_EXE% --path "%~dp0godot_project" --main-pack "%~dp0godot_project\scenes\main.tscn"

echo.
echo [EXIT] Game closed with code %ERRORLEVEL%
pause
