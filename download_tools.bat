@echo off
color 0F
setlocal EnableDelayedExpansion

REM ####################################################
REM #               INSTALLATION SCRIPT                #
REM ####################################################
REM Choose between Visual Studio Build Tools or LLVM

:main
echo.
echo [[34mSELECT[0m] Choose installation option:
echo [[36mY[0m] Install Visual Studio Build Tools
echo [[35mN[0m] Install LLVM Tools (including lld)
set /p CONFIRM=^> [Y/N] 
if /i "!CONFIRM!" == "Y" (
    call :install_vs_buildtools
    if %errorlevel% neq 0 goto :error
) else if /i "!CONFIRM!" == "N" (
    call :install_llvm_tools
    if %errorlevel% neq 0 goto :error
) else (
    echo [[31mERROR[0m] Invalid selection. Exiting.
    goto :error
)

REM Success path
echo.
echo [[32mSUCCESS[0m] Installation completed successfully!
pause
exit /b 0

:install_vs_buildtools
REM Download the installer
echo [[34mDOWNLOAD[0m] Getting Build Tools installer...
bitsadmin /transfer "VSBuildTools" /priority high "https://aka.ms/vs/17/release/vs_buildtools.exe" "%TEMP%\vs_buildtools.exe"
if %errorlevel% neq 0 (
    echo [[31mERROR[0m] Failed to download Build Tools installer.
    exit /b 1
)

REM Install silently to user directory
set "INSTALL_DIR=%USERPROFILE%\VSBuildTools"
echo [[34mINSTALL[0m] Installing Build Tools to %INSTALL_DIR%...
"%TEMP%\vs_buildtools.exe" --quiet --wait --norestart --nocache --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --installPath "%INSTALL_DIR%"
if %errorlevel% neq 0 (
    echo [[31mERROR[0m] Build Tools installation failed.
    exit /b 1
)

REM Find and run vcvarsall.bat to configure environment
echo [[34mSEARCH[0m] Locating vcvarsall.bat...
set "VCVARSALL_PATH="
for /f "delims=" %%i in ('where /r "%INSTALL_DIR%" vcvarsall.bat') do (
    set "VCVARSALL_PATH=%%i"
    goto :found_vcvarsall
)
for /f "delims=" %%i in ('where /r "C:\Program Files\Microsoft Visual Studio" vcvarsall.bat') do (
    set "VCVARSALL_PATH=%%i"
    goto :found_vcvarsall
)
for /f "delims=" %%i in ('where /r "C:\Program Files (x86)\Microsoft Visual Studio" vcvarsall.bat') do (
    set "VCVARSALL_PATH=%%i"
    goto :found_vcvarsall
)
echo [[31mERROR[0m] Could not find vcvarsall.bat.
exit /b 1
:found_vcvarsall
call "%VCVARSALL_PATH%" x64 >nul 2>&1
if %errorlevel% neq 0 (
    echo [[31mERROR[0m] Failed to configure Visual Studio environment.
    exit /b 1
)
echo [[32mSUCCESS[0m] Build Tools environment configured.
exit /b 0

:install_llvm_tools
REM Install LLVM tools from the provided EXE
set "LLVM_URL=https://github.com/llvm/llvm-project/releases/download/llvmorg-20.1.0/LLVM-20.1.0-win64.exe"
echo [[34mDOWNLOAD[0m] Getting LLVM installer...
bitsadmin /transfer "LLVMInstaller" /priority high "%LLVM_URL%" "%TEMP%\llvm-installer.exe"
if %errorlevel% neq 0 (
    echo [[31mERROR[0m] Failed to download LLVM installer.
    exit /b 1
)

REM Install to user directory
set "LLVM_INSTALL_DIR=%USERPROFILE%\LLVM"
echo [[34mINSTALL[0m] Installing LLVM Tools to %LLVM_INSTALL_DIR%...
"%TEMP%\llvm-installer.exe" /VERYSILENT /SUPPRESSMSGBOXES /LOG=%TEMP%\llvm-install.log /DIR="%LLVM_INSTALL_DIR%"
if %errorlevel% neq 0 (
    echo [[31mERROR[0m] LLVM Tools installation failed.
    exit /b 1
)
echo [[32mSUCCESS[0m] LLVM Tools installed successfully.
exit /b 0

:error
echo.
echo [[31mFATAL ERROR[0m] Installation failed. Check output for details.
echo.
pause
exit /b 1