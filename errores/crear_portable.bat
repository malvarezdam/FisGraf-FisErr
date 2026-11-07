@echo off
rem FisGraf y FisErr (c) 2026 Martin Alvarez de Araya. Uso gratuito citando al autor (ver LICENSE).
rem Parte de errores: FisErr portable con su manual.
rem   portable\FisErr\  y  dist\Errores_FisErr-portable.zip
rem El .exe se compila con la biblioteca de C estatica (+crt-static): funciona en
rem cualquier Windows sin instalar Visual C++ Redistributable.
rem Usa errores\Manual_FisErr.pdf (se regenera con manual\compilar.bat).
setlocal
rem Raiz del repositorio (este archivo esta en errores\).
cd /d "%~dp0.."

echo [Errores 1/2] Compilando FisErr portable...
set "RUSTFLAGS=-C target-feature=+crt-static"
cargo build --release -p fiserr --target-dir target\portable
if errorlevel 1 goto fallo
set "RUSTFLAGS="

echo [Errores 2/2] Armando la carpeta...
set "OUT=portable\FisErr"
if exist "%OUT%" rmdir /s /q "%OUT%"
mkdir "%OUT%\FisErr"
copy /y target\portable\release\fiserr.exe "%OUT%\FisErr\FisErr.exe" >nul
copy /y errores\Manual_FisErr.pdf          "%OUT%\FisErr\Manual FisErr.pdf" >nul
copy /y errores\LEEME_portable.txt         "%OUT%\LEEME.txt" >nul
copy /y LICENSE                            "%OUT%\LICENSE" >nul

if not exist dist mkdir dist
del /q dist\Errores_FisErr-portable.zip 2>nul
tar -a -c -f dist\Errores_FisErr-portable.zip -C "%OUT%" FisErr LEEME.txt LICENSE
if errorlevel 1 goto fallo
echo   dist\Errores_FisErr-portable.zip
if /i not "%~1"=="sinpausa" pause
exit /b 0

:fallo
echo No se pudo crear FisErr portable.
if /i not "%~1"=="sinpausa" pause
exit /b 1
