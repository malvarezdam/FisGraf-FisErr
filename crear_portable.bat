@echo off
rem Crea la carpeta "portable" con FisGraf, FisErr, ejemplos y manuales (sin fisgraf-cli).
rem Los .exe se compilan con la biblioteca de C estatica (+crt-static), asi que
rem funcionan en cualquier Windows sin instalar Visual C++ Redistributable.
setlocal
cd /d "%~dp0"

echo [1/3] Compilando los ejecutables portables...
set "RUSTFLAGS=-C target-feature=+crt-static"
cargo build --release --workspace --bins --target-dir target\portable
if errorlevel 1 (
    echo La compilacion fallo.
    if /i not "%~1"=="sinpausa" pause
    exit /b 1
)
set "RUSTFLAGS="
set "BIN=target\portable\release"

echo [2/3] Compilando los manuales...
call manual\compilar.bat sinpausa
if errorlevel 1 echo Se usaran los PDF que ya estan en la carpeta principal.
cd /d "%~dp0"

echo [3/3] Armando la carpeta portable...
if exist portable rmdir /s /q portable
mkdir portable\FisGraf portable\FisErr
copy /y "%BIN%\fisgraf.exe"     "portable\FisGraf\FisGraf.exe" >nul
copy /y "%BIN%\fiserr.exe"      "portable\FisErr\FisErr.exe" >nul
rem Manual sin fisgraf-cli ni Claude; si no se pudo compilar, el completo.
set "MANUAL=manual\manual_portable.pdf"
if not exist "%MANUAL%" set "MANUAL=MANUAL.pdf"
copy /y "%MANUAL%"              "portable\FisGraf\Manual FisGraf.pdf" >nul
copy /y MANUAL_FisErr.pdf       "portable\FisErr\Manual FisErr.pdf" >nul
copy /y manual\LEEME_portable.txt "portable\LEEME.txt" >nul
robocopy graficos\examples\proyectos portable\FisGraf\examples\proyectos /e /njh /njs /nfl /ndl >nul
robocopy graficos\examples\datos     portable\FisGraf\examples\datos     /e /njh /njs /nfl /ndl >nul
robocopy graficos\datos_de_prueba    portable\FisGraf\datos_de_prueba    /e /njh /njs /nfl /ndl >nul

rem Un zip por programa en dist\ (son los que se suben a la Release de GitHub).
if not exist dist mkdir dist
del /q dist\FisGraf-portable.zip dist\FisErr-portable.zip 2>nul
tar -a -c -f dist\FisGraf-portable.zip -C portable FisGraf LEEME.txt
tar -a -c -f dist\FisErr-portable.zip  -C portable FisErr LEEME.txt

echo Listo: carpeta portable, dist\FisGraf-portable.zip y dist\FisErr-portable.zip
if /i not "%~1"=="sinpausa" pause
