@echo off
rem FisGraf y FisErr (c) 2026 Martin Alvarez de Araya. Uso gratuito citando al autor (ver LICENSE).
rem Parte de graficos: FisGraf portable con su manual, ejemplos y datos de prueba.
rem   portable\FisGraf\  y  dist\Graficos_FisGraf-portable.zip
rem El .exe se compila con la biblioteca de C estatica (+crt-static): funciona en
rem cualquier Windows sin instalar Visual C++ Redistributable.
rem Usa graficos\Manual_FisGraf.pdf (se regenera con manual\compilar.bat).
setlocal
rem Raiz del repositorio (este archivo esta en graficos\).
cd /d "%~dp0.."

echo [Graficos 1/2] Compilando FisGraf portable...
set "RUSTFLAGS=-C target-feature=+crt-static"
cargo build --release -p fisgraf --bin fisgraf --target-dir target\portable
if errorlevel 1 goto fallo
set "RUSTFLAGS="

echo [Graficos 2/2] Armando la carpeta...
set "OUT=portable\FisGraf"
if exist "%OUT%" rmdir /s /q "%OUT%"
mkdir "%OUT%\FisGraf"
copy /y target\portable\release\fisgraf.exe "%OUT%\FisGraf\FisGraf.exe" >nul
copy /y graficos\Manual_FisGraf.pdf         "%OUT%\FisGraf\Manual FisGraf.pdf" >nul
copy /y graficos\LEEME_portable.txt         "%OUT%\LEEME.txt" >nul
copy /y LICENSE                             "%OUT%\LICENSE" >nul
robocopy graficos\examples\proyectos "%OUT%\FisGraf\examples\proyectos" /e /njh /njs /nfl /ndl >nul
robocopy graficos\examples\datos     "%OUT%\FisGraf\examples\datos"     /e /njh /njs /nfl /ndl >nul
robocopy graficos\datos_de_prueba    "%OUT%\FisGraf\datos_de_prueba"    /e /njh /njs /nfl /ndl >nul

if not exist dist mkdir dist
del /q dist\Graficos_FisGraf-portable.zip 2>nul
tar -a -c -f dist\Graficos_FisGraf-portable.zip -C "%OUT%" FisGraf LEEME.txt LICENSE
if errorlevel 1 goto fallo
echo   dist\Graficos_FisGraf-portable.zip
if /i not "%~1"=="sinpausa" pause
exit /b 0

:fallo
echo No se pudo crear FisGraf portable.
if /i not "%~1"=="sinpausa" pause
exit /b 1
