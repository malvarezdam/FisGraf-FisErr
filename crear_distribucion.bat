@echo off
rem FisGraf y FisErr (c) 2026 Martin Alvarez de Araya. Uso gratuito citando al autor (ver LICENSE).
rem Crea en dist\ todo lo que se publica en la Release de GitHub, separado en dos partes:
rem   Graficos_FisGraf-portable.zip                              (graficos\crear_portable.bat)
rem   Errores_FisErr-portable.zip                                (errores\crear_portable.bat)
rem Para publicarlos (con GitHub CLI):
rem   gh release create vX.Y.Z dist\* --title "FisGraf y FisErr vX.Y.Z" --notes "..."
rem Los scripts se llaman con ruta completa: con NoDefaultCurrentDirectoryInExePath
rem (lo define Claude Code), cmd no los encuentra por su nombre solo.
setlocal
cd /d "%~dp0"
if exist dist rmdir /s /q dist
echo [Manuales] Compilando...
call "%~dp0manual\compilar.bat" sinpausa
if errorlevel 1 echo Se usaran los PDF que ya estan en el repositorio.
call "%~dp0graficos\crear_portable.bat" sinpausa
if errorlevel 1 goto fallo
call "%~dp0errores\crear_portable.bat" sinpausa
if errorlevel 1 goto fallo
call "%~dp0ia\crear_paquetes.bat" sinpausa
if errorlevel 1 goto fallo
echo.
echo Listo: dist\
dir /b "%~dp0dist"
if /i not "%~1"=="sinpausa" pause
exit /b 0

:fallo
echo No se pudo crear la distribucion.
if /i not "%~1"=="sinpausa" pause
exit /b 1
