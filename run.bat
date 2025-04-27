@echo off
color 0F
setlocal EnableDelayedExpansion

:main_install
echo.
echo ####################################################
echo #                   DEPLOY SCRIPT                  #
echo ####################################################
echo.
echo [[34mSTATUS[0m] Deployment initiated
echo [[34mCHECK[0m] Checking dependencies
echo.

call :check_for_rust
set "RUST_STATUS=!errorlevel!"

if !RUST_STATUS! equ 0 (
    echo [[32mOK[0m] Rust is installed
) else (
    echo [[31mERROR[0m] Rust is missing or incorrectly installed
)

if !RUST_STATUS! neq 0 (
    echo.
    echo [[33mWARNING[0m] Missing dependencies detected
    echo [[33mQUESTION[0m] Install Rust? [Y/N]
    set /p CONFIRM=^> 
    if /i "!CONFIRM!" == "Y" (
        call :install_rust && cls || goto :error
    ) else (
        echo.
        echo [[33mWARNING[0m] Proceeding without dependencies may cause failures
        echo [[33mQUESTION[0m] Continue? [Y/N]
        set /p BUILD_ANYWAY=^> 
        if /i "!BUILD_ANYWAY!" == "N" exit /b 1
    )
)

set "PATH=%USERPROFILE%\.cargo\bin;%USERPROFILE%\VSBuildTools\VC\Auxiliary\Build;%USERPROFILE%\LLVM\bin;!PATH!"

echo.
echo [[34mSTATUS[0m] Environment configured successfully
echo [[34mACTION[0m] Starting deployment...
call :main_run

:error
echo.
echo [[31mERROR[0m] Deployment failed
echo.
echo [[33mOPTION[0m] Would you like to:
echo [[33m1[0m] Reinstall Rust
echo [[33m2[0m] Re-run deploy script
echo [[33m3[0m] Exit
set /p OPTION=^> 
if "%OPTION%" == "1" (
    call :install_rust && (
        echo [[32mOK[0m] Reinstallation successful
        goto :main_install
    ) || (
        echo [[31mERROR[0m] Reinstallation failed
        pause >nul & exit /b 1
    )
)else if "%OPTION%" == "2" (
    call :main_run & goto :error
) else (
    echo [[31mERROR[0m] Exiting deployment
    pause >nul & exit /b 1
)

:check_for_rust
echo [[34mSTATUS[0m] Checking for existing Rust installation...
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" exit /b 0
exit /b 1

:install_rust
cls
echo.
echo ####################################################
echo #               RUST INSTALLER SCRIPT              #
echo ####################################################
echo.
echo [[34mSELECT[0m] Target triple:
echo [[32mD[0m] x86_64-pc-windows-msvc (default)
echo [[32mG[0m] x86_64-pc-windows-gnu
echo [Custom] Enter your own target triple
set /p TARGET_TRIPLE=^> 

set "INSTALL=0"
if not defined TARGET_TRIPLE (
    echo [[34mSTATUS[0m] Setting toolchain to default
    set "TARGET_TRIPLE=x86_64-pc-windows-msvc"
) else if /i "%TARGET_TRIPLE%" == "D" (
    set "TARGET_TRIPLE=x86_64-pc-windows-msvc"
) else if /i "%TARGET_TRIPLE%" == "G" (
    set "TARGET_TRIPLE=x86_64-pc-windows-gnu"
)

set "RUSTUP_URL=https://win.rustup.rs/x86_64"
if "%PROCESSOR_ARCHITECTURE%" == "x86" set "RUSTUP_URL=https://win.rustup.rs/i686"

echo [[34mDOWNLOAD[0m] Installer...
bitsadmin /transfer "RustInstall" /dynamic /priority high "%RUSTUP_URL%" "%TEMP%\rustup-init.exe" || (
    echo [[31mERROR[0m] Failed to download installer
    exit /b 1
)

"%TEMP%\rustup-init.exe" -y --default-toolchain "stable-%TARGET_TRIPLE%" -t %TARGET_TRIPLE%

if errorlevel 1 (
    echo [[31mERROR[0m] Installation failed
    exit /b 1
)

if not "%TARGET_TRIPLE%"== "x86_64-pc-windows-gnu" (
    echo [[34mCONFIG[0m] Setting LLD linker...
    (
        echo [build]
        echo rustflags = ["-C", "linker=rust-lld"]
    ) >> "%USERPROFILE%\.cargo\config.toml"
)
rustup component add cargo rustc 2>nul

echo [[32mOK[0m] Rust installed successfully
set "PATH=%USERPROFILE%\.cargo\bin;!PATH!"
echo [[34mSTATUS[0m] Added Rust to system PATH
exit /b 0

:main_run
cls
echo.
echo ####################################################
echo #                   RUN SCRIPT                     #
echo ####################################################
echo.

echo [[34mCHECK[0m] Verifying Rust installation...
if not exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
    echo [[31mERROR[0m] Rust/Cargo not found, install that first.
    pause >nul & exit /b 1
)

echo [[34mCHECK[0m] Validating project directory...
if not exist "Cargo.toml" (
    echo [[31mERROR[0m] No Cargo.toml found. Run from project root.
    echo Current directory: %CD%
    pause >nul & exit /b 1
)

echo [[34mCOMPILING[0m] Compiling...
cargo build || (
    echo [[31mERROR[0m] Build failed
    pause >nul & exit /b 1
)

for /f "tokens=2 delims== " %%a in ('findstr "^name *= *" Cargo.toml') do set "CRATE_NAME=%%a"
set "CRATE_NAME=%CRATE_NAME:"=%"

if not exist "target\debug\%CRATE_NAME%.exe" (
    echo [[31mERROR[0m] Executable not found
    pause >nul & exit /b 1
)

echo [[32mOK[0m] Deployment completed successfully!
start "" "target\debug\%CRATE_NAME%.exe"
pause >nul & exit /b 0