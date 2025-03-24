@echo off
color 0F
setlocal EnableDelayedExpansion
REM ####################################################
REM #                   DEPLOY SCRIPT                  #
REM ####################################################
:main
echo.
echo [[34mSTATUS[0m] Deployment initiated
echo [[34mCHECK[0m] Checking dependencies
echo.
REM Check Rust installation status
call :check_for_rust
set "RUST_STATUS=!errorlevel!"
REM Display dependency status
if !RUST_STATUS! equ 0 (
    echo [[32mOK[0m] Rust is installed
) else (
    echo [[31mERROR[0m] Rust is missing or incorrectly installed
)
REM Determine if installation is needed
if !RUST_STATUS! neq 0 (
    echo.
    echo [[33mWARNING[0m] Missing dependencies detected
    echo [[33mQUESTION[0m] Install Rust? [Y/N]
    set /p CONFIRM=^> 
    if /i "!CONFIRM!" == "Y" (
        call :install_rust
		cls
        if %errorlevel% neq 0 goto :error
    ) else (
        echo.
        echo [[33mWARNING[0m] Proceeding without dependencies may cause failures
        echo [[33mQUESTION[0m] Continue? [Y/N]
        set /p BUILD_ANYWAY=^> 
        if /i "!BUILD_ANYWAY!" == "N" exit /b 1
    )
)
REM Set environment variables
set "PATH=%USERPROFILE%\.cargo\bin;%USERPROFILE%\VSBuildTools\VC\Auxiliary\Build;%USERPROFILE%\LLVM\bin;!PATH!"
set "VCToolsRedistDir=%USERPROFILE%\VSBuildTools\VC\Redist"
REM Final deployment
echo.
echo [[34mSTATUS[0m] Environment configured successfully
echo [[34mACTION[0m] Starting deployment...
call :deploy_project
if %errorlevel% neq 0 goto :error
exit /b 0

:error
echo.
echo [[31mERROR[0m] Deployment failed
echo.
echo [[33mOPTION[0m] Would you like to:
echo [[33m1[0m] Reinstall Rust
echo [[33m2[0m] Exit
set /p OPTION=^> 
if "%OPTION%" == "1" (
    call :install_rust
    if %errorlevel% equ 0 (
        echo [[32mOK[0m] Reinstallation successful
        goto :main
    ) else (
        echo [[31mERROR[0m] Reinstallation failed
        pause
        exit /b 1
    )
) else (
    echo [[31mERROR[0m] Exiting deployment
    pause
    exit /b 1
)
:check_for_rust
REM ####################################################
REM #               RUST INSTALLER SCRIPT              #
REM ####################################################
echo [[34mSTATUS[0m] Checking for existing Rust installation...
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
    echo [[32mOK[0m] Rust is already installed
    echo.
    echo [[33mWARNING[0m] Proceeding will overwrite existing installation
    echo [[33mQUESTION[0m] Reinstall Rust? [Y/N]
    set /p CONFIRM=^> 
    if /i "!CONFIRM!" == "Y" (
        call :install_rust
		cls
    ) else (
        echo [[34mSTATUS[0m] Continuing without installation
    )
) else (
    echo [[31mERROR[0m] Rust not found
    echo [[34mACTION[0m] Installing Rust...
    call :install_rust
	cls
)
exit /b 0

:install_rust
cls
echo.
echo ####################################################
echo #               RUST INSTALLER SCRIPT              #
echo ####################################################
echo.
echo You will need 'Admin' for everything other than the option "G"
echo Also it compiles a tiny bit faster so I suggest using that, I use that myself
echo.
echo [[34mSELECT[0m] Select target triple:
echo [[32mD[0m] x86_64-pc-windows-msvc (default)
echo [[32mG[0m] x86_64-pc-windows-gnu
echo [Custom] Enter your own target triple
set /p TARGET_TRIPLE=^> 
REM Process target selection
if "%TARGET_TRIPLE%" == "" (
    set "TARGET_TRIPLE=x86_64-pc-windows-msvc"
    echo [[34mSTATUS[0m] Using default target
) else if "%TARGET_TRIPLE%" == "d" (
    set "TARGET_TRIPLE=x86_64-pc-windows-msvc"
    echo [[34mSTATUS[0m] Selected target: %TARGET_TRIPLE%
) else if "%TARGET_TRIPLE%" == "g" (
    set "TARGET_TRIPLE=x86_64-pc-windows-gnu"
    echo [[34mSTATUS[0m] Selected target: %TARGET_TRIPLE%
) else (
    set "TARGET_TRIPLE=%TARGET_TRIPLE%"
    echo [[34mSTATUS[0m] Using custom target: %TARGET_TRIPLE%
)
REM Detect system architecture for installer URL
set "RUSTUP_URL=https://win.rustup.rs/x86_64"
if "%PROCESSOR_ARCHITECTURE%" == "x86" (
    set "RUSTUP_URL=https://win.rustup.rs/i686"
)
REM Download installer with progress
echo [[34mDOWNLOAD[0m] Installer...
bitsadmin /transfer "RustInstall" /dynamic /priority high "%RUSTUP_URL%" "%TEMP%\rustup-init.exe"
if %errorlevel% neq 0 (
    echo [[31mERROR[0m] Failed to download installer
    exit /b 1
)
REM Install Rust with correct default toolchain
echo [[34mINSTALL[0m] Installing...
"%TEMP%\rustup-init.exe" -y --default-toolchain "stable-%TARGET_TRIPLE%" -t %TARGET_TRIPLE%
if %errorlevel% neq 0 (
    echo [[31mERROR[0m] Installation failed
    exit /b 1
)
REM sometimes some components does not get installed correctly, in that case add it manually
REM this mainly happens if the user chose the "G" installation
rustup component add cargo
rustup component add rustc

REM Update environment variables
set "PATH=%USERPROFILE%\.cargo\bin;!PATH!"
echo [[32mOK[0m] Rust installed successfully
echo [[34mSTATUS[0m] Added Rust to system PATH
exit /b 0

:deploy_project
echo.
echo [[34mDEPLOY[0m] Deploying project...
echo.
REM Check for Visual Studio Build Tools
if not exist "%USERPROFILE%\VSBuildTools" (
    echo [[33mWARNING[0m] Visual Studio Build Tools not found
    echo [[33mINFO[0m] That is the default but installing other kinds of linking chains is fine too
    echo [[33mQUESTION[0m] Continue? [Y/N]
    set /p CONFIRM=^> 
    if /i "!CONFIRM!" == "N" exit /b 1
)
REM Clean target directory
if exist "target\" (
    echo [[34mSTATUS[0m] Target directory found
    echo [[34mQUESTION[0m] Clean before building? [Y/N]
    set /p CONFIRM=^> 
    if /i "!CONFIRM!" == "Y" (
        echo [[34mCLEAN[0m] Cleaning...
        cargo clean
    ) else if /i "!CONFIRM!" == "N" (
        echo [[34mSTATUS[0m] Skipping clean
    ) else (
        echo [[31mERROR[0m] Invalid input
        exit /b 1
    )
)
REM Build project
echo [[34mCOMPILING[0m] Compiling...
cargo build
if %errorlevel% neq 0 (
    echo [[31mERROR[0m] Build failed
    exit /b 1
)
REM Verify executable
for /f "tokens=2 delims== " %%a in ('findstr /R /C:"^name *= *" Cargo.toml') do (
    set "CRATE_NAME=%%a"
)
set "CRATE_NAME=%CRATE_NAME:"=%"
if not exist "target\debug\%CRATE_NAME%.exe" (
    echo [[31mERROR[0m] Executable not found
    exit /b 1
)
echo [[32mOK[0m] Deployment completed successfully!
echo [[34mLAUNCHING[0m] Launching application...
start "" "target\debug\%CRATE_NAME%.exe"
exit /b 0