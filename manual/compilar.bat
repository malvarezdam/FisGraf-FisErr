@echo off
rem Compila los manuales y los deja en la carpeta principal:
rem   manual.tex        -> MANUAL.pdf         (FisGraf, FisErr y fisgraf-cli)
rem   manual_fiserr.tex -> MANUAL_FisErr.pdf  (solo FisErr)
rem Usa lualatex directamente (latexmk necesita Perl). Dos pasadas: el indice y las
rem referencias se resuelven en la segunda.
rem Con el argumento "sinpausa" no espera una tecla al terminar (lo usa crear_portable.bat).
cd /d "%~dp0"
for %%M in (manual manual_fiserr) do (
    lualatex -interaction=nonstopmode -halt-on-error %%M.tex >nul
    if errorlevel 1 goto fallo
    lualatex -interaction=nonstopmode -halt-on-error %%M.tex >nul
    if errorlevel 1 goto fallo
)
copy /y manual.pdf "..\MANUAL.pdf" >nul
copy /y manual_fiserr.pdf "..\MANUAL_FisErr.pdf" >nul
del /q *.aux *.log *.toc *.out 2>nul
echo Listo: MANUAL.pdf y MANUAL_FisErr.pdf
if /i not "%~1"=="sinpausa" pause
exit /b 0

:fallo
echo La compilacion fallo: revisa el archivo .log en la carpeta manual
if /i not "%~1"=="sinpausa" pause
exit /b 1
