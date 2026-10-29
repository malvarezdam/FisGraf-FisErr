@echo off
rem Compila el manual (manual.tex) y deja MANUAL.pdf en la carpeta principal.
cd /d "%~dp0"
latexmk -lualatex -interaction=nonstopmode -halt-on-error manual.tex
if errorlevel 1 (
    echo La compilacion fallo: revisa manual.log
    pause
    exit /b 1
)
copy /y manual.pdf "..\MANUAL.pdf" >nul
latexmk -c >nul
echo Listo: MANUAL.pdf
pause
