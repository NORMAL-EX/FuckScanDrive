@echo off
echo Building FuckScanDrive...
echo.

echo [1/3] Building Hook DLL...
cargo build --release --manifest-path hook_dll/Cargo.toml
if %errorlevel% neq 0 (
    echo Failed to build Hook DLL
    exit /b 1
)

echo.
echo [2/3] Copying Hook DLL to target directory...
copy /Y hook_dll\target\release\fuck_scan_hook.dll target\release\
if %errorlevel% neq 0 (
    echo Failed to copy Hook DLL
    exit /b 1
)

echo.
echo [3/3] Building main application...
cargo build --release
if %errorlevel% neq 0 (
    echo Failed to build main application
    exit /b 1
)

echo.
echo ========================================
echo Build completed successfully!
echo ========================================
echo.
echo Output files:
echo   - target\release\fuck_scan_drive.exe
echo   - target\release\fuck_scan_hook.dll
echo.
echo Don't forget to configure fuck.ini before running!
echo.
pause
