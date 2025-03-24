@echo off
color 0F
setlocal EnableDelayedExpansion

REM ####################################################
REM #                   DEPLOY SCRIPT                   #
REM ####################################################
REM This script checks for Rust/Cargo and handles user-mode installations

:main
echo [[34mINFO[0m] Starting deployment...
echo.

REM Check Rust status
call :check_for_rust
set "RUST_STATUS=!errorlevel!"
if !RUST_STATUS! equ 0 (
    echo [[32mInstalled[0m] Rust is already installed.
) else (
    echo [[31mMissing[0m] Rust is not installed.
)

REM Determine if we need to install anything
if !RUST_STATUS! neq 0 (
    echo The following dependencies are missing: Rust
    echo.
    echo [[33mWARNING[0m] Install missing dependencies? [Y/N]
    set /p CONFIRM=^> 
    if /i "!CONFIRM!" == "Y" (
        call :install_rust
        if %errorlevel% neq 0 goto :error
    ) else (
        echo.
        echo [[33mWARNING[0m] Proceeding without installing may cause build failures.
        echo Do you want to try building anyway? [Y/N]
        set /p BUILD_ANYWAY=^> 
        if /i "!BUILD_ANYWAY!" == "N" (
            echo Exiting deployment.
            exit /b 1
        )
    )
)

REM Set up environment variables for user directories
set "PATH=%USERPROFILE%\.cargo\bin;%USERPROFILE%\VSBuildTools\VC\Auxiliary\Build;%USERPROFILE%\LLVM\bin;!PATH!"
set "VCToolsRedistDir=%USERPROFILE%\VSBuildTools\VC\Redist"

REM Final deployment
echo.
echo [[34mINFO[0m] Deployment environment ready. Proceeding to deploy...
echo.
call :deploy_project
if %errorlevel% neq 0 goto :error

:error
echo.
echo [[31mERROR[0m] Deployment failed.
echo.
pause
exit /b 1

REM Helper Functions
:check_for_rust
echo [[34mCHECK[0m] Verifying Rust installation...
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
    exit /b 0
) else (
    exit /b 1
)

:install_rust
echo [[34mINSTALL[0m] Downloading Rust...
REM Detect system architecture
set "RUSTUP_URL=https://win.rustup.rs/x86_64"
if "%PROCESSOR_ARCHITECTURE%" == "x86" (
    set "RUSTUP_URL=https://win.rustup.rs/i686"
)

REM Download installer
echo [[34mINFO[0m] Transferring installer...
bitsadmin /transfer "RustInstall" /priority high "%RUSTUP_URL%" "%TEMP%\rustup-init.exe"
if %errorlevel% neq 0 (
    echo [[31mERROR[0m] Failed to download Rust installer.
    exit /b 1
)

REM Install silently to user directory
echo [[34mINSTALL[0m] Installing Rust...
"%TEMP%\rustup-init.exe" -y --default-toolchain stable -t rust,rust-src
if %errorlevel% neq 0 (
    echo [[31mERROR[0m] Rust installation failed.
    exit /b 1
)

REM Update PATH for current session
set "PATH=%USERPROFILE%\.cargo\bin;!PATH!"
echo [[32mSUCCESS[0m] Rust installed successfully.
exit /b 0

:deploy_project
echo.
echo [[34mDEPLOY[0m] Preparing project deployment...
echo.

REM Check for required build tools
if not exist "%USERPROFILE%\VSBuildTools" (
    echo [[31mERROR[0m] Missing Visual Studio Build Tools
    echo [[33mWARNING[0m] Proceeding without them may cause build failures
    echo Do you want to continue anyway? [Y/N]
    set /p BUILD_ANYWAY=^> 
    if /i "!BUILD_ANYWAY!" == "N" (
        echo Exiting deployment.
        exit /b 1
    )
)

REM Check for target directory and clean if needed
if exist "target\" (
    echo Target directory found. Clean before building? [Y/N]
    set /p CONFIRM=^> 
    if /i "!CONFIRM!" == "Y" (
        echo [[34mCLEAN[0m] Removing existing artifacts...
        cargo clean
    ) else if /i "!CONFIRM!" == "N" (
        echo Skipping clean to use cached build
    ) else (
        echo [[31mERROR[0m] Invalid input. Exiting.
        exit /b 1
    )
) else (
    echo Target directory does not exist. Skipping clean.
)

echo [[34mBUILD[0m] Starting compilation...
cargo build
if %errorlevel% neq 0 (
    echo [[31mERROR[0m] Build failed.
    exit /b 1
)

REM Extract crate name from Cargo.toml
for /f "tokens=2 delims== " %%a in ('findstr /R /C:"^name *= *" Cargo.toml') do (
    set "CRATE_NAME=%%a"
)
set "CRATE_NAME=%CRATE_NAME:"=%"  REM Remove quotes

REM Check executable existence
if not exist "target\debug\%CRATE_NAME%.exe" (
    echo [[31mERROR[0m] Executable not found. Deployment failed.
    exit /b 1
)

echo [[32mSUCCESS[0m] Deployment completed! Launching application...
start "" "target\debug\%CRATE_NAME%.exe"
exit /b 0
