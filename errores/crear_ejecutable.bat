@echo off
rem Compila FisErr (calculadora de errores) y deja FisErr.exe en esta carpeta.
cd /d "%~dp0"
echo Compilando FisErr (la primera vez tarda unos minutos)...
cargo build --release -p fiserr
if errorlevel 1 (
    echo.
    echo La compilacion fallo. Revisa que Rust este instalado: https://rustup.rs
    pause
    exit /b 1
)
copy /y "..\target\release\fiserr.exe" "FisErr.exe" >nul
echo.
echo Listo: FisErr.exe esta en %~dp0
pause
