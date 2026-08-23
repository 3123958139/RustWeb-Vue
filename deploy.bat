@echo off
setlocal
title RustWeb One-Click Deploy
cd /d %~dp0

echo ========================================
echo   RustWeb One-Click Deploy: build + package + start
echo ========================================

rem Build frontends in PARALLEL (max 3 concurrent, see build-frontends.ps1).
rem Order still matters: frontends must be built BEFORE the backend, because the
rem frontend dist is embedded INTO the exe at compile time
rem (cargo build --release --features embedded, see src/embedded_assets.rs).
echo Building 4 frontends in parallel (3 concurrent)...
powershell -NoProfile -ExecutionPolicy Bypass -File build-frontends.ps1
if errorlevel 1 (
    echo.
    echo [FAILED] One or more frontend builds failed. Please run "npm install" first.
    pause
    exit /b 1
)

rem [8] Build backend with embedded frontend assets (single exe)
echo [8] Building backend (cargo build --release --features embedded)...
call cargo build --release --features embedded
if errorlevel 1 (
    echo.
    echo [FAILED] Backend build failed. Please check the Rust toolchain.
    pause
    exit /b 1
)

rem [8/8] Assemble deploy directory (single exe + runtime-only files)
echo [8/8] Assembling deploy directory...
if not exist deploy mkdir deploy
copy /y target\release\fj200c-backend.exe deploy\ >nul

rem Generate .env if it does not exist
if not exist deploy\.env (
    echo PORT=3000> deploy\.env
    echo DATABASE_URL=sqlite://fj200c.db>> deploy\.env
    echo JWT_EXPIRATION=86400>> deploy\.env
    echo RUST_LOG=info>> deploy\.env
    echo CORS_ORIGINS=http://localhost:3000,http://127.0.0.1:3000>> deploy\.env
)

rem Copy per-role config files and create csv directory
rem fj200c_information role config
if not exist config-fj200c_information.ini (
    echo [WARN] config-fj200c_information.ini not found. A default one will be created.
    echo [Mock]> config-fj200c_information.ini
    echo InProcess = true>> config-fj200c_information.ini
    echo FeederMode = false>> config-fj200c_information.ini
    echo.>> config-fj200c_information.ini
    echo [Connection0]>> config-fj200c_information.ini
    echo Enabled = true>> config-fj200c_information.ini
    echo ComPort = COM3>> config-fj200c_information.ini
    echo BaudRate = 115200>> config-fj200c_information.ini
    echo DataBits = 8>> config-fj200c_information.ini
    echo StopBits = 1>> config-fj200c_information.ini
    echo Parity = 0>> config-fj200c_information.ini
    echo FlowControl = false>> config-fj200c_information.ini
    echo.>> config-fj200c_information.ini
    echo [CSV]>> config-fj200c_information.ini
    echo Enabled = true>> config-fj200c_information.ini
    echo Dir = csv>> config-fj200c_information.ini
)
copy /y config-fj200c_information.ini deploy\ >nul
if not exist deploy\csv mkdir deploy\csv

rem fj200c_main role config (ECU/ADAM/DYNO 三路串口测控)
if not exist config-fj200c_main.ini (
    echo [WARN] config-fj200c_main.ini not found. A default one will be created.
    echo [general]> config-fj200c_main.ini
    echo name = fj200c_main>> config-fj200c_main.ini
    echo version = 0.0.1>> config-fj200c_main.ini
    echo.>> config-fj200c_main.ini
    echo [COM]>> config-fj200c_main.ini
    echo Count = 5>> config-fj200c_main.ini
    echo.>> config-fj200c_main.ini
    echo [COM0]>> config-fj200c_main.ini
    echo PORTNAME = COM101>> config-fj200c_main.ini
    echo BaudRate = 115200>> config-fj200c_main.ini
    echo DataBits = 8>> config-fj200c_main.ini
    echo Parity = 0>> config-fj200c_main.ini
    echo StopBits = 1>> config-fj200c_main.ini
    echo TimeoutMs = 100>> config-fj200c_main.ini
    echo.>> config-fj200c_main.ini
    echo [COM1]>> config-fj200c_main.ini
    echo PORTNAME = COM103>> config-fj200c_main.ini
    echo BaudRate = 9600>> config-fj200c_main.ini
    echo DataBits = 8>> config-fj200c_main.ini
    echo Parity = 0>> config-fj200c_main.ini
    echo StopBits = 1>> config-fj200c_main.ini
    echo TimeoutMs = 100>> config-fj200c_main.ini
    echo.>> config-fj200c_main.ini
    echo [COM2]>> config-fj200c_main.ini
    echo PORTNAME = COM105>> config-fj200c_main.ini
    echo BaudRate = 9600>> config-fj200c_main.ini
    echo DataBits = 8>> config-fj200c_main.ini
    echo Parity = 0>> config-fj200c_main.ini
    echo StopBits = 1>> config-fj200c_main.ini
    echo TimeoutMs = 100>> config-fj200c_main.ini
    echo.>> config-fj200c_main.ini
    echo [COM3]>> config-fj200c_main.ini
    echo PORTNAME = COM107>> config-fj200c_main.ini
    echo BaudRate = 115200>> config-fj200c_main.ini
    echo DataBits = 8>> config-fj200c_main.ini
    echo Parity = 0>> config-fj200c_main.ini
    echo StopBits = 1>> config-fj200c_main.ini
    echo TimeoutMs = 100>> config-fj200c_main.ini
    echo.>> config-fj200c_main.ini
    echo [COM4]>> config-fj200c_main.ini
    echo PORTNAME = COM109>> config-fj200c_main.ini
    echo BaudRate = 115200>> config-fj200c_main.ini
    echo DataBits = 8>> config-fj200c_main.ini
    echo Parity = 0>> config-fj200c_main.ini
    echo StopBits = 1>> config-fj200c_main.ini
    echo TimeoutMs = 100>> config-fj200c_main.ini
    echo.>> config-fj200c_main.ini
    echo [MOCK]>> config-fj200c_main.ini
    echo SimulationMenu = true>> config-fj200c_main.ini
    echo.>> config-fj200c_main.ini
    echo [REPORT]>> config-fj200c_main.ini
    echo StatePoints = 30000,31000,32000,33000,34000,35000,36000,37000,38000,39000,40000,41000,42000,43000,44000,45000,46000,47000,48000,49000,50000,51000,52000,53000>> config-fj200c_main.ini
    echo.>> config-fj200c_main.ini
    echo [CSV]>> config-fj200c_main.ini
    echo Dir = csv>> config-fj200c_main.ini
)
copy /y config-fj200c_main.ini deploy\ >nul

echo [8/8] Deployment complete!
echo ----------------------------------------
echo   Deploy folder: %~dp0deploy
echo   Start:        double-click deploy\fj200c-backend.exe
echo   Fj200c_information:      http://localhost:3000/fj200c_information
echo   Fj200c_main:             http://localhost:3000/fj200c_main
echo   Admin:                   http://localhost:3000/admin
echo   Mario:                   http://localhost:3000/mario
echo ----------------------------------------

cd deploy

rem Read port from .env (default 3000)
set PORT=3000
if exist .env (
    for /f "usebackq tokens=1,* delims==" %%a in (".env") do (
        if /i "%%a"=="PORT" if not "%%b"=="" set "PORT=%%b"
    )
)

rem Check whether the port is already in use
set OCCUPIED_PID=
for /f "tokens=5" %%p in ('netstat -ano ^| findstr /c:":%PORT% " ^| findstr /c:"LISTENING"') do (
    if not "%%p"=="0" if not defined OCCUPIED_PID set "OCCUPIED_PID=%%p"
)

if not "%OCCUPIED_PID%"=="" (
    echo [WARN] Port %PORT% is already in use by PID %OCCUPIED_PID%.
    choice /c YN /m "Kill the process occupying port %PORT%? Y=kill, N=abort"
    if errorlevel 2 (
        echo [ABORTED] Port %PORT% is still occupied. Server was NOT started.
        pause
        exit /b 1
    )
    echo [INFO] Killing PID %OCCUPIED_PID% ...
    taskkill /F /T /PID %OCCUPIED_PID% >nul 2>&1
    if errorlevel 1 (
        echo [FAILED] Unable to kill PID %OCCUPIED_PID%. Please stop it manually and retry.
        pause
        exit /b 1
    )
    echo [OK] PID %OCCUPIED_PID% killed. Port %PORT% is free now.
    timeout /t 1 /nobreak >nul
)

start "RustWeb" cmd /k fj200c-backend.exe
echo Server started. Press any key to close this window...
pause >nul
endlocal
