@echo off
color 0F
echo.
echo ####################################################
echo #                   RUN SCRIPT                      #
echo ####################################################
echo.

REM --- Check for Rust installation ---
where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo Error: Rust/Cargo is not installed!
    echo Run 'install.bat' before running app.
    pause
    exit /b 1
)

REM --- Check for project directory ---
if not exist "Cargo.toml" (
    echo Error: No Cargo.toml found!
    echo Please run this script from your Rust project directory.
    pause
    exit /b 1
)

REM --- Run the application ---
echo.
echo Starting your application...
cargo run

REM --- Handle errors ---
if %errorlevel% neq 0 (
    echo.
    echo Error: Application failed to run. Check the output above.
    echo This might be triggered by the Application.
    pause
    exit /b %errorlevel%
)

REM --- Success message ---
echo.
echo Application exited successfully!
pause