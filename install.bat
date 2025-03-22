@echo off
color 0F
setlocal EnableDelayedExpansion
REM ####################################################
REM #                   DEPLOY SCRIPT                   #
REM ####################################################
REM This script checks for Rust/Cargo and Visual Studio Build Tools,
REM installs missing dependencies, and deploys the Rust project.
REM --- Helper Functions ---
goto :main
:deploy_project
echo Deploying your Rust project...
echo.
REM Check if target directory exists before prompting
if exist "target\" (
    echo Target directory found. Do you want to clean it before building?
    set /p CONFIRM=^> [Y/N] 
    if /i "!CONFIRM!" == "Y" (
        echo Cleaning project...
        cargo clean
    ) else if /i "!CONFIRM!" == "N" (
        echo Skipping clean to use cached build
    ) else (
        echo Invalid input. Exiting.
        exit /b 1
    )
) else (
    echo Target directory does not exist. Skipping clean.
)
echo Building application...
cargo build
if %errorlevel% neq 0 (
    echo Build failed. Exiting...
    exit /b 1
)
REM Extract crate name from Cargo.toml
for /f "tokens=2 delims== " %%a in ('findstr /R /C:"^name *= *" Cargo.toml') do (
    set "CRATE_NAME=%%a"
)
set "CRATE_NAME=%CRATE_NAME:"=%"  REM Remove quotes
REM Check executable existence
if not exist "target\debug\%CRATE_NAME%.exe" (
    echo Executable not found. Deployment failed.
    exit /b 1
)
echo Deployment successful! Running the application...
target\debug\%CRATE_NAME%.exe
exit /b 0

:check_for_rust
echo Checking for Rust installation...
where cargo >nul 2>&1
exit /b %errorlevel%
:install_rust
echo Installing Rust...
REM Detect system architecture
set "RUSTUP_URL=https://win.rustup.rs/x86_64.exe"
if "%PROCESSOR_ARCHITECTURE%" == "x886" (
    set "RUSTUP_URL=https://win.rustup.rs/i686.exe"
)
REM Download installer
echo Downloading Rust installer...
bitsadmin /transfer "RustInstall" /priority high "%RUSTUP_URL%" "%TEMP%\rustup-init.exe"
if %errorlevel% neq 0 (
    echo Failed to download Rust installer. Exiting.
    exit /b 1
)
REM Install silently
echo Installing Rust...
"%TEMP%\rustup-init.exe" -y --default-toolchain stable -t rust,rust-src
if %errorlevel% neq 0 (
    echo Rust installation failed. Exiting.
    exit /b 1
)
REM Update PATH for current session
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
echo Rust installed successfully.
exit /b 0
:check_for_vs_buildtools
echo Checking for Visual Studio Build Tools...
where cl >nul 2>&1
exit /b %errorlevel%
:install_vs_buildtools
echo Installing Visual Studio Build Tools...
REM Download the installer
echo Downloading Build Tools...
bitsadmin /transfer "VSBuildTools" /priority high "https://aka.ms/vs/17/release/vs_buildtools.exe" "%TEMP%\vs_buildtools.exe"
if %errorlevel% neq 0 (
    echo Failed to download Build Tools installer. Exiting.
    exit /b 1
)
REM Install silently with required components
echo Installing Build Tools...
"%TEMP%\vs_buildtools.exe" --quiet --wait --norestart --nocache --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended
if %errorlevel% neq 0 (
    echo Build Tools installation failed. Exiting.
    exit /b 1
)
REM Find and run vcvarsall.bat to configure environment
set "VCVARSALL_PATH="
for /f "delims=" %%i in ('where /r "C:\Program Files (x86)\Microsoft Visual Studio" vcvarsall.bat') do (
    set "VCVARSALL_PATH=%%i"
    goto :found_vcvarsall
)
echo Could not find vcvarsall.bat. Exiting.
exit /b 1
:found_vcvarsall
call "%VCVARSALL_PATH%" x64 >nul 2>&1
if %errorlevel% neq 0 (
    echo Failed to configure Visual Studio environment. Exiting.
    exit /b 1
)
echo Build Tools environment configured.
exit /b 0
:main
REM Check dependencies first
echo Starting deployment...
echo.
REM Check Rust status
call :check_for_rust
set "RUST_STATUS=!errorlevel!"
if !RUST_STATUS! equ 0 (
    echo [Installed] Rust is already installed.
) else (
    echo [Missing] Rust is not installed.
)
REM Check VS Build Tools status
call :check_for_vs_buildtools
set "VS_STATUS=!errorlevel!"
if !VS_STATUS! equ 0 (
    echo [Installed] Visual Studio Build Tools are already installed.
) else (
    echo [Missing] Visual Studio Build Tools are not installed.
)
echo.
REM Determine missing dependencies
set "MISSING="
if !RUST_STATUS! neq 0 set "MISSING=!MISSING!Rust, "
if !VS_STATUS! neq 0 set "MISSING=!MISSING!Visual Studio Build Tools, "
REM Trim trailing comma
if defined MISSING set "MISSING=!MISSING:~0,-2!"
REM Prompt user if any dependencies are missing
if defined MISSING (
    echo The following dependencies are missing: %MISSING%
    echo.
    echo Press Y to install them or N to skip installation.
    set /p CONFIRM=^> [Y/N] 
    if /i "!CONFIRM!" == "Y" (
        set "DO_INSTALL=Y"
    ) else if /i "!CONFIRM!" == "N" (
        echo.
        echo WARNING: Proceeding without installing may cause build failures.
        echo Do you want to try building anyway?
        set /p BUILD_ANYWAY=^> [Y/N] 
        if /i "!BUILD_ANYWAY!" == "Y" (
            set "DO_INSTALL=N"
        ) else (
            echo Exiting deployment.
            goto :error
        )
    ) else (
        echo Invalid input. Exiting.
        goto :error
    )
) else (
    set "DO_INSTALL=Y"  REM No dependencies missing, proceed normally
)

REM Install missing dependencies only if user agreed to install
if defined DO_INSTALL (
    if "!DO_INSTALL!" == "Y" (
        if !RUST_STATUS! neq 0 (
            echo Installing Rust...
            call :install_rust
            if %errorlevel% neq 0 goto :error
        )
        if !VS_STATUS! neq 0 (
            echo Installing Visual Studio Build Tools...
            call :install_vs_buildtools
            if %errorlevel% neq 0 goto :error
        )
    )
)

REM Final deployment
echo.
echo Deployment environment ready. Proceeding to deploy...
echo.
call :deploy_project
if %errorlevel% neq 0 goto :error

:error
echo.
echo Deployment failed.
pause
exit /b 0