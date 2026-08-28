@echo off
REM CloseOption SSID Export Script (Windows)
REM
REM Usage:
REM   scripts\export_closeoption_ssid.bat [--browser chrome] [--save]
REM
REM Environment:
REM   CLOSEOPTION_SSID - Session ID format: token|sid|demo|public_code|hidden_code

@setlocal enabledelayedexpansion

set "BROWSER=chrome"
set "SAVE_SESSION=0"

REM Parse arguments
:argloop
if "%~1"=="" goto :endargs
if "%~1"=="--browser" set "BROWSER=%~2" & shift & shift & goto :argloop
if "%~1"=="-b" set "BROWSER=%~2" & shift & shift & goto :argloop
if "%~1"=="--save" set "SAVE_SESSION=1" & shift & goto :argloop
if "%~1"=="-s" set "SAVE_SESSION=1" & shift & goto :argloop
if "%~1"=="--help" set "SHOW_HELP=1" & shift & goto :argloop
if "%~1"=="-h" set "SHOW_HELP=1" & shift & goto :argloop
shift & goto :argloop
:endargs

if defined SHOW_HELP (
    echo CloseOption SSID Export Tool (Windows)
    echo.
    echo Usage: export_closeoption_ssid.bat [--browser chrome] [--save]
    echo.
    echo Options:
    echo   --browser, -b  Browser to check (chrome, firefox, edge, brave)
    echo   --save, -s     Save entered SSID to file
    echo   --help, -h     Show this help
    echo.
    echo SSID Format: token|sid|demo|public_code|hidden_code
    goto :eof
)

echo CloseOption SSID Export Tool
echo Platform: Windows
echo Browser: %BROWSER%
echo.

REM Check for saved session
set "SESSION_FILE=%USERPROFILE%\.closeoption_session.json"
if exist "%SESSION_FILE%" (
    echo Found saved session: %SESSION_FILE%
    type "%SESSION_FILE%"
    echo.
    echo To export:
    echo   set CLOSEOPTION_SSID=...
    goto :eof
)

REM Check browser installation
set "BROWSER_PATH="
if "%BROWSER%"=="chrome" (
    set "BROWSER_PATH=%LOCALAPPDATA%\Google\Chrome\User Data"
) else if "%BROWSER%"=="firefox" (
    set "BROWSER_PATH=%APPDATA%\Mozilla\Firefox\Profiles"
) else if "%BROWSER%"=="edge" (
    set "BROWSER_PATH=%LOCALAPPDATA%\Microsoft\Edge\User Data"
) else if "%BROWSER%"=="brave" (
    set "BROWSER_PATH=%LOCALAPPDATA%\BraveSoftware\Brave-Browser\User Data"
)

if exist "%BROWSER_PATH%" (
    echo Browser found: %BROWSER_PATH%
    echo.
    echo Note: Use Python script for browser extraction:
    echo   python scripts\export_closeoption_ssid.py --browser %BROWSER%
) else (
    echo Browser not found at expected location
)

echo.
echo === SSID Export Instructions ===
echo.
echo Method 1: Browser DevTools
echo --------------------------
echo 1. Open https://www.closeoption.com
echo 2. Press F12 to open DevTools
echo 3. Go to Application tab
echo 4. Find Cookies for closeoption.com
echo 5. Extract: token, sid, publicCode, hiddenCode, isDemo
echo.
echo SSID Format:
echo   token|sid|demo|public_code|hidden_code
echo.
echo Example:
echo   abc123token|xyz789sid|true|pub_code123|hid_code456
echo.

if "%SAVE_SESSION%"=="1" (
    echo.
    set /p SSID="Enter SSID (token|sid|demo|public_code|hidden_code): "
    
    if not "%SSID%"=="" (
        for /f "tokens=1-5 delims=|" %%a in ("%SSID%") do (
            (
                echo {
                echo   "token": "%%a",
                echo   "sid": "%%b",
                echo   "demo": %%c,
                echo   "public_code": "%%d",
                echo   "hidden_code": "%%e"
                echo }
            ) > "%SESSION_FILE%"
            echo Saved to: %SESSION_FILE%
        )
    )
)

goto :eof
