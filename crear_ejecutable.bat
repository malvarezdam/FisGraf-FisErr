@echo off
rem Compila FisGraf en modo optimizado y deja FisGraf.exe en esta carpeta.
rem Doble clic en este archivo, o ejecutalo desde una terminal.
cd /d "%~dp0"
echo Compilando FisGraf (la primera vez tarda unos minutos)...
cargo build --release
if errorlevel 1 (
    echo.
    echo La compilacion fallo. Revisa que Rust este instalado: https://rustup.rs
    pause
    exit /b 1
)
copy /y "target\release\fisgraf.exe" "FisGraf.exe" >nul
echo.
echo Listo: FisGraf.exe esta en %~dp0
echo Puedes crear un acceso directo en el escritorio (clic derecho ^> Enviar a ^> Escritorio).
pause
