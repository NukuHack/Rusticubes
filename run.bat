@echo off
color 0F
echo.
echo ####################################################
echo #                   RUN SCRIPT                     #
echo ####################################################
echo.

REM --- System Checks ---
echo [[34mCHECK[0m] Verifying Rust installation...
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
    echo [[34mOK[0m] Cargo found.
) else (
    echo [[31mERROR[0m] Rust/Cargo not found. Please run 'install.bat' first.
    pause
    exit /b 1
)

echo [[34mCHECK[0m] Validating project directory...
if not exist "Cargo.toml" (
    echo [[31mERROR[0m] No 'Cargo.toml' found. Run this script from your project root.
    echo Current directory: %CD%
    pause
    exit /b 1
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