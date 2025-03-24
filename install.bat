@echo off
color 0F
setlocal EnableDelayedExpansion
REM ####################################################
REM #                   DEPLOY SCRIPT                   #
REM ####################################################

:main
echo.
echo [[34m [2K [34mDEPLOYMENT INITIATED[0m]
echo [[34m [2K [34mChecking dependencies[0m]
echo.

REM Check Rust installation status
call :check_for_rust
set "RUST_STATUS=!errorlevel!"

REM Display dependency status
if !RUST_STATUS! equ 0 (
    echo [[32m✓[0m] Rust is installed
) else (
    echo [[31m✗[0m] Rust is missing
)

REM Determine if installation is needed
if !RUST_STATUS! neq 0 (
    echo.
    echo [[33m![0m] Missing dependencies detected
    echo [[33m![0m] Install Rust? [Y/N]
    set /p CONFIRM=^> 
    if /i "!CONFIRM!" == "Y" (
        call :install_rust
        if %errorlevel% neq 0 goto :error
    ) else (
        echo.
        echo [[33m![0m] Proceeding without dependencies may cause failures
        echo Do you want to continue? [Y/N]
        set /p BUILD_ANYWAY=^> 
        if /i "!BUILD_ANYWAY!" == "N" exit /b 1
    )
)

REM Set environment variables
set "PATH=%USERPROFILE%\.cargo\bin;%USERPROFILE%\VSBuildTools\VC\Auxiliary\Build;%USERPROFILE%\LLVM\bin;!PATH!"
set "VCToolsRedistDir=%USERPROFILE%\VSBuildTools\VC\Redist"

REM Final deployment
echo.
echo [[34m[0m] Environment configured successfully
echo [[34m[0m] Starting deployment...
echo.
call :deploy_project
if %errorlevel% neq 0 goto :error
exit /b 0

:error
echo.
echo [[31mERROR[0m] Deployment failed
echo.
echo [[33m[0m] Would you like to:
echo [[33m1[0m] Reinstall Rust
echo [[33m2[0m] Exit
set /p OPTION=^> 
if "%OPTION%" == "1" (
    call :install_rust
    if %errorlevel% equ 0 (
        echo [[32m✓[0m] Reinstallation successful
        goto :main
    ) else (
        echo [[31m✗[0m] Reinstallation failed
        pause
        exit /b 1
    )
) else (
    echo [[31m[0m] Exiting deployment
    pause
    exit /b 1
)

:check_for_rust
echo [[34m[0m] Verifying Rust installation...
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
    exit /b 0
) else (
    exit /b 1
)

:install_rust
echo [[34m[0m] Installing Rust...
echo.
echo [[34m[0m] Select target triple:
echo [[32mD[0m] x86_64-pc-windows-msvc (default)
echo [[32mG[0m] x86_64-pc-windows-gnu
echo [Custom] Enter your own target triple
set /p TARGET_TRIPLE=^> 

REM Process target selection
if "%TARGET_TRIPLE%" == "" (
    set "TARGET_TRIPLE=x86_64-pc-windows-msvc"
    echo [[34m[0m] Using default target
) else if "%TARGET_TRIPLE%" == "D" (
    set "TARGET_TRIPLE=x86_64-pc-windows-msvc"
    echo [[34m[0m] Selected target: %TARGET_TRIPLE%
) else if "%TARGET_TRIPLE%" == "G" (
    set "TARGET_TRIPLE=x86_64-pc-windows-gnu"
    echo [[34m[0m] Selected target: %TARGET_TRIPLE%
) else (
    set "TARGET_TRIPLE=%TARGET_TRIPLE%"
    echo [[34m[0m] Using custom target: %TARGET_TRIPLE%
)

REM Detect system architecture for installer URL
set "RUSTUP_URL=https://win.rustup.rs/x86_64"
if "%PROCESSOR_ARCHITECTURE%" == "x86" (
    set "RUSTUP_URL=https://win.rustup.rs/i686"
)

REM Download installer
echo [[34m[0m] Downloading installer...
bitsadmin /transfer "RustInstall" /priority high "%RUSTUP_URL%" "%TEMP%\rustup-init.exe"
if %errorlevel% neq 0 (
    echo [[31m✗[0m] Failed to download installer
    exit /b 1
)

REM Install Rust silently
echo [[34m[0m] Installing Rust...
"%TEMP%\rustup-init.exe" -y --default-toolchain stable -t %TARGET_TRIPLE%
if %errorlevel% neq 0 (
    echo [[31m✗[0m] Rust installation failed
    exit /b 1
)

REM Add target if necessary
if not "%TARGET_TRIPLE%" == "!RUSTUP_DEFAULT_TARGET!" (
    echo [[34m[0m] Adding target %TARGET_TRIPLE%...
    rustup target add %TARGET_TRIPLE%
    if %errorlevel% neq 0 (
        echo [[31m✗[0m] Failed to add target
        exit /b 1
    )
)

REM Update PATH
set "PATH=%USERPROFILE%\.cargo\bin;!PATH!"
echo [[32m✓[0m] Rust installed successfully
exit /b 0

:deploy_project
echo.
echo [[34m[0m] Deploying project...
echo.

REM Check for Visual Studio Build Tools
if not exist "%USERPROFILE%\VSBuildTools" (
    echo [[33m![0m] Missing Visual Studio Build Tools detected
    echo Do you want to continue? [Y/N]
    set /p CONFIRM=^> 
    if /i "!CONFIRM!" == "N" exit /b 1
)

REM Clean target directory if needed
if exist "target\" (
    echo [[34m[0m] Target directory found
    echo Clean before building? [Y/N]
    set /p CONFIRM=^> 
    if /i "!CONFIRM!" == "Y" (
        echo [[34m[0m] Cleaning...
        cargo clean
    ) else if /i "!CONFIRM!" == "N" (
        echo Skipping clean
    ) else (
        echo [[31m✗[0m] Invalid input
        exit /b 1
    )
)

REM Build project
echo [[34m[0m] Compiling...
cargo build
if %errorlevel% neq 0 (
    echo [[31m✗[0m] Build failed
    exit /b 1
)

REM Verify executable
for /f "tokens=2 delims== " %%a in ('findstr /R /C:"^name *= *" Cargo.toml') do (
    set "CRATE_NAME=%%a"
)
set "CRATE_NAME=%CRATE_NAME:"=%"

if not exist "target\debug\%CRATE_NAME%.exe" (
    echo [[31m✗[0m] Executable not found
    exit /b 1
)

echo [[32m✓[0m] Deployment completed successfully!
echo [[34m[0m] Launching application...
start "" "target\debug\%CRATE_NAME%.exe"
exit /b 0