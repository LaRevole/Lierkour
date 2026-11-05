@echo off
rem Portable launch: the database, posters and screenshots live in .\data
rem next to the executable instead of %APPDATA%\com.myfilmlibrary.desktop.
setlocal
set "MY_FILM_LIBRARY_DATA_DIR=%~dp0data"
start "" "%~dp0myfilmlibrary-app.exe" %*
endlocal
