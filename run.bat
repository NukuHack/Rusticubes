@echo off
color 0F
echo.
echo ####################################################
echo #                   RUN SCRIPT                     #
echo ####################################################
echo.

REM --- System Checks ---
echo [[34mCHECK[0m] Verifying Rust installation...
where cargo >nul 2>nul
if %errorlevel% neq 0 (
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

REM --- Execution ---
echo.
echo [[34mRUN[0m] Starting application...
cargo run

REM --- Post-run Handling ---
if %errorlevel% neq 0 (
    echo.
    echo [[31mERROR[0m] Application failed to execute properly.
    echo Check the build output above for details.
    echo.
    pause
    exit /b %errorlevel%
)

echo.
echo [[32mSUCCESS[0m] Application exited successfully!
echo.
pause