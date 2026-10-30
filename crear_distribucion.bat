@echo off
rem Crea todo lo que se publica en la Release de GitHub, en la carpeta dist\:
rem   FisGraf-portable.zip, FisErr-portable.zip, FisGraf-para-Claude.zip y fisgraf-cli.exe
rem Para publicarlos (con GitHub CLI):
rem   gh release create vX.Y.Z dist\* --title "FisGraf y FisErr vX.Y.Z" --notes "..."
setlocal
cd /d "%~dp0"
call "%~dp0crear_portable.bat" sinpausa
if errorlevel 1 goto fallo
call "%~dp0crear_skill_portable.bat" sinpausa
if errorlevel 1 goto fallo
echo.
echo Listo: dist\
dir /b dist
if /i not "%~1"=="sinpausa" pause
exit /b 0

:fallo
echo No se pudo crear la distribucion.
if /i not "%~1"=="sinpausa" pause
exit /b 1
