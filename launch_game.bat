@echo off
:: Wasteland Project Launch Script
:: Godot 4.6 + Rust GDExtension

setlocal enabledelayedexpansion

:: Configuration
set "GODOT_EXE=godot.exe"
set "PROJECT_DIR=%~dp0godot_project"
set "GDEXTENSION_DIR=%PROJECT_DIR%\bin"
set "SCENE=%PROJECT_DIR%\scenes\main.tscn"
set "OUTPUT_LOG=%~dp0logs\game.log"

:: Create logs directory
if not exist "%~dp0logs" mkdir "%~dp0logs"

:: Check if Godot is available
where %GODOT_EXE% >nul 2>&1
if %errorlevel% equ 0 (
    echo [LAUNCHER] Godot found in PATH
) else (
    echo [LAUNCHER] Looking for Godot in common locations...
    
    :: Check common installation paths
    set "GODOT_PATHS="
    if exist "C:\Program Files\Godot Engine\Godot_4.6\Godot_v4.6-stable_win64.exe" set "GODOT_EXE=C:\Program Files\Godot Engine\Godot_4.6\Godot_v4.6-stable_win64.exe"
    if exist "D:\Program Files\Godot Engine\Godot_4.6\Godot_v4.6-stable_win64.exe" set "GODOT_EXE=D:\Program Files\Godot Engine\Godot_4.6\Godot_v4.6-stable_win64.exe"
    if exist "%~dp0bin\godot.exe" set "GODOT_EXE=%~dp0bin\godot.exe"
    
    if not defined GODOT_PATHS (
        echo [ERROR] Godot not found!
        echo Please install Godot 4.6 and add to PATH, or place godot.exe in the bin directory.
        pause
        exit /b 1
    )
)

:: Verify GDExtension
if not exist "%GDEXTENSION_DIR%\wasteland_gdextension.dll" (
    echo [WARNING] GDExtension not found at %GDEXTENSION_DIR%\wasteland_gdextension.dll
    echo Attempting to build...
    cd /d "%~dp0"
    cargo build --release
    if %errorlevel% equ 0 (
        copy "target\release\wasteland_gdextension.dll" "%GDEXTENSION_DIR%"
    ) else (
        echo [ERROR] Failed to build GDExtension
        pause
        exit /b 1
    )
)

:: Launch game
echo [LAUNCHER] Starting Wasteland Project...
echo [LAUNCHER] Godot: %GODOT_EXE%
echo [LAUNCHER] Project: %PROJECT_DIR%
echo [LAUNCHER] Log: %OUTPUT_LOG%

:: Run in headless mode for testing
:: %GODOT_EXE% --headless --script "%PROJECT_DIR%\tests\smoke_test.gd" > "%OUTPUT_LOG%" 2>&1

:: Run in normal mode
%GODOT_EXE% --path "%PROJECT_DIR%" --main-pack "%SCENE%"

echo [LAUNCHER] Game exited with code %errorlevel%
type "%OUTPUT_LOG%"

pause