@echo off
rem FisGraf y FisErr (c) 2026 Martin Alvarez de Araya. Uso gratuito citando al autor (ver LICENSE).
rem Compila los manuales y deja cada uno en su parte:
rem   manual_portable.tex -> graficos\Manual_FisGraf.pdf (FisGraf y FisErr)
rem   manual_fiserr.tex   -> errores\Manual_FisErr.pdf   (solo FisErr)
rem Usa lualatex directamente (latexmk necesita Perl). Dos pasadas: el indice y las
rem referencias se resuelven en la segunda.
rem Con el argumento "sinpausa" no espera una tecla al terminar (lo usa crear_distribucion.bat).
cd /d "%~dp0"
for %%M in (manual_fiserr manual_portable) do (
    lualatex -interaction=nonstopmode -halt-on-error %%M.tex >nul
    if errorlevel 1 goto fallo
    lualatex -interaction=nonstopmode -halt-on-error %%M.tex >nul
    if errorlevel 1 goto fallo
)
copy /y manual_portable.pdf "..\graficos\Manual_FisGraf.pdf" >nul
copy /y manual_fiserr.pdf "..\errores\Manual_FisErr.pdf" >nul
del /q *.aux *.log *.toc *.out 2>nul
echo Listo: manual\Manual_completo.pdf, graficos\Manual_FisGraf.pdf y errores\Manual_FisErr.pdf
if /i not "%~1"=="sinpausa" pause
exit /b 0

:fallo
echo La compilacion fallo: revisa el archivo .log en la carpeta manual
if /i not "%~1"=="sinpausa" pause
exit /b 1
