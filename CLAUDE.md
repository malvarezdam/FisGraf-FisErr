# FisGraf y FisErr: instrucciones para Claude

## Autor y derechos
- **Dueño y autor:** Martín Álvarez de Araya (GitHub `malvarezdam`), el usuario con quien trabajas. Todo el proyecto es obra suya.
- **Licencia** (`LICENSE`): propia, no de código abierto.
  - Uso gratuito para cualquier fin, citando siempre al autor.
  - Prohibido venderlo o cobrar por él, presentarlo como propio, copiar, modificar o redistribuir el código fuente, crear obras derivadas y usarlo para entrenar IA.
  - No la cambies ni la debilites sin que él lo pida.
- **Avisos de autoría:** mantenlos en todo lo que se publique. Están en `LICENSE`, en la cabecera de los `.rs` principales, en `authors` de Cargo, en la ventana «Acerca de» (`graficos/src/gui/about.rs`) y en los LEEME de los zips.
- **Cita:** «Álvarez de Araya, M. (2026). FisGraf y FisErr (v1.0). https://github.com/malvarezdam/FisGraf-FisErr».

## Estructura: dos partes
| Carpeta | Paquete | Qué es |
|---|---|---|
| `graficos/` | `fisgraf` | **FisGraf**: gráficos de datos con ajustes, incertidumbres y LaTeX. Contiene también la **biblioteca con toda la lógica numérica** (`src/lib.rs`), que usan FisErr. Su arquitectura y su historial están en `graficos/PLAN.md`. |
| `errores/` | `fiserr` | **FisErr**: calculadora de propagación de errores. Reutiliza `fisgraf::propagation` y `fisgraf::gui::calculator`. |
| `manual/` | | Fuente LaTeX de los manuales y `MANUAL.md`. |

**Parte de IA (skills, plugin, `fisgraf-cli` y servidor MCP):** ya no está en GitHub porque se vende en otra plataforma. Su copia es `venta/FisGraf_IA_v1.0.zip`, ignorada por git. No la vuelvas a subir al repositorio ni a las Releases.

## Comandos
- **Compilar, probar y revisar:** `cargo build --workspace`, `cargo test --workspace` y `cargo clippy --workspace --all-targets`.
- **Abrir las apps:** `cargo run -p fisgraf` y `cargo run -p fiserr`.
- **Ejecutables normales:** `graficos/crear_ejecutable.bat` y `errores/crear_ejecutable.bat` (ignorados en git).
- **Manuales:** `manual/compilar.bat` genera `graficos/Manual_FisGraf.pdf` y `errores/Manual_FisErr.pdf`.
- **Distribución:** cada parte arma lo suyo en `dist/` con `+crt-static` en `target/portable`.
  - `graficos/crear_portable.bat` → `Graficos_FisGraf-portable.zip`.
  - `errores/crear_portable.bat` → `Errores_FisErr-portable.zip`.
  - `crear_distribucion.bat` hace todo (manuales incluidos). Después se publica con `gh release create vX.Y.Z dist/* …`.
- **Versión:** está en `[workspace.package]` del `Cargo.toml` raíz.
- **Scripts `.bat`:** llaman a otros scripts con ruta completa (`call "%~dp0otro.bat"`). Claude Code define `NoDefaultCurrentDirectoryInExePath`, y entonces cmd no encuentra un script por su nombre solo. `.gitattributes` fuerza CRLF en `.bat` y `.ps1`.

## Reglas de trabajo
- **Al terminar un cambio:**
  - la compilación no debe dar errores;
  - `cargo test --workspace` debe quedar en verde;
  - clippy no debe dar advertencias nuevas.

  Después anótalo en `graficos/PLAN.md` y haz un commit.
- **Commits intermedios:** si la sesión puede cortarse a mitad de un cambio grande, haz commits intermedios con el prefijo `WIP:`.
- **Idioma:** interfaz, comentarios de usuario y mensajes en **español**; identificadores de código en inglés.
- **Gráficos:** todo texto que aparezca en el gráfico (etiquetas, leyendas, ecuaciones) se escribe en LaTeX.
- **Lógica numérica:** vive solo en `graficos/src`. FisErr la usa; no la copies.

## Entorno
- **Sistema:** Windows 11, Rust 1.97 y MiKTeX 25.12, con `lualatex`, `pdflatex`, `latexmk`, `pdftoppm` y `pdftocairo` en el PATH.
- **Repositorio:** https://github.com/malvarezdam/FisGraf-FisErr (rama `main`). Las descargas van en sus Releases, no en git.
- **Barras invertidas en Rust:** al escribir código Rust con barras invertidas a través de un heredoc de bash, se pierden las dobles (`\\`). Usa la herramienta Write/Edit o cadenas crudas `r"…"`.
- **Perl con texto en español:** con `perl -pi`, usa `-CSD -Mutf8`. Sin `-Mutf8`, las tildes del propio script se corrompen.
- **Nombres de archivo en Windows:** no distinguen mayúsculas. `manual/MANUAL.pdf` y `manual/manual.pdf` son el mismo archivo; por eso los PDF publicados se llaman `Manual_*.pdf`.
