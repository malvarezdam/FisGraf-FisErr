@echo off
rem Compila FisGraf en modo optimizado y deja FisGraf.exe en esta carpeta.
cd /d "%~dp0"
echo Compilando FisGraf (la primera vez tarda unos minutos)...
cargo build --release -p fisgraf
if errorlevel 1 (
    echo.
    echo La compilacion fallo. Revisa que Rust este instalado: https://rustup.rs
    pause
    exit /b 1
)
copy /y "..\target\release\fisgraf.exe" "FisGraf.exe" >nul
echo.
echo Listo: FisGraf.exe esta en %~dp0
pause
