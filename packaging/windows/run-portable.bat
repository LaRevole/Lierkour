@echo off
rem Portable launch: the database, posters and screenshots live in .\data
rem next to the executable instead of %APPDATA%\com.lierkour.desktop.
setlocal
set "LIERKOUR_DATA_DIR=%~dp0data"
set "MY_FILM_LIBRARY_DATA_DIR=%~dp0data"
start "" "%~dp0lierkour-app.exe" %*
endlocal
