# FisGraf y FisErr: instrucciones para Claude

## Autor y derechos
- **Dueño y autor:** Martín Álvarez de Araya (GitHub `malvarezdam`), el usuario con quien trabajas. Todo el proyecto es obra suya.
- **Licencia** (`LICENSE`): propia, no de código abierto.
  - Uso gratuito para cualquier fin, citando siempre al autor.
  - Prohibido venderlo o cobrar por él, presentarlo como propio, copiar, modificar o redistribuir el código fuente, crear obras derivadas y usarlo para entrenar IA.
  - No la cambies ni la debilites sin que él lo pida.
- **Avisos de autoría:** mantenlos en todo lo que se publique. Están en `LICENSE`, en la cabecera de los `.rs` principales, en `authors` de Cargo, en `plugin.json` y `marketplace.json`, en la ventana «Acerca de» (`graficos/src/gui/about.rs`), en los LEEME de los zips, en `ia/guia_ia.md`, en SKILL.md y en las instrucciones del servidor MCP.
- **Cita:** «Álvarez de Araya, M. (2026). FisGraf y FisErr (v1.0). https://github.com/malvarezdam/FisGraf-FisErr».

## Estructura: tres partes
| Carpeta | Paquete | Qué es |
|---|---|---|
| `graficos/` | `fisgraf` | **FisGraf**: gráficos de datos con ajustes, incertidumbres y LaTeX. Contiene también la **biblioteca con toda la lógica numérica** (`src/lib.rs`), que usan las otras dos partes. Su arquitectura y su historial están en `graficos/PLAN.md`. |
| `errores/` | `fiserr` | **FisErr**: calculadora de propagación de errores. Reutiliza `fisgraf::propagation` y `fisgraf::gui::calculator`. |
| `ia/` | `fisgraf-cli` | **Parte de IA**. Ver la lista siguiente. |
| `manual/` | | Fuente LaTeX de los manuales, `MANUAL.md` y `Manual_completo.pdf`. |

La parte de IA (`ia/`) contiene:
- `fisgraf-cli/`: comandos JSON (`main.rs`) y servidor MCP (`mcp.rs`).
- `claude/`: plugin de Claude Code con la skill `fisgraf`.
- `chatgpt/`: configuración para Codex.
- `gemini/`: extensión de Gemini CLI.
- `guia_ia.md`: instrucciones comunes, que se copian como `AGENTS.md` y `GEMINI.md`.
- `instalar.ps1` y `crear_paquetes.*`.

**Reglas de la parte de IA:**
- La skill `ia/claude/skills/fisgraf/` es la única copia en git. Es a la vez el plugin (publicado por `.claude-plugin/marketplace.json`, que va en la raíz) y el contenido de `IA_Claude.zip`. Hay una copia instalada en `~/.claude/skills/fisgraf/` (con el `.exe`): si cambias la skill, actualízala.
- La skill no debe tener rutas de este computador. Busca el `.exe` junto al SKILL.md, en `~/.fisgraf/` o en el PATH; si no lo encuentra, lo descarga de la última Release (`IA_fisgraf-cli.exe`).
- `mcp.rs` incluye `ia/claude/skills/fisgraf/referencia.md` con `include_str!`.
- `instalar.ps1` y `crear_paquetes.ps1` deben quedar solo con caracteres ASCII.

## Comandos
- **Compilar, probar y revisar:** `cargo build --workspace`, `cargo test --workspace` y `cargo clippy --workspace --all-targets`.
- **Abrir las apps:** `cargo run -p fisgraf` y `cargo run -p fiserr`. **Herramienta:** `cargo build --release -p fisgraf-cli`.
- **Ejecutables normales:** `graficos/crear_ejecutable.bat` y `errores/crear_ejecutable.bat` (ignorados en git).
- **Manuales:** `manual/compilar.bat` genera `manual/Manual_completo.pdf` (todo), `graficos/Manual_FisGraf.pdf` (sin la parte de IA) y `errores/Manual_FisErr.pdf`.
- **Distribución:** cada parte arma lo suyo en `dist/` con `+crt-static` en `target/portable`.
  - `graficos/crear_portable.bat` → `Graficos_FisGraf-portable.zip`.
  - `errores/crear_portable.bat` → `Errores_FisErr-portable.zip`.
  - `ia/crear_paquetes.bat` → `IA_Claude.zip`, `IA_ChatGPT-Codex.zip`, `IA_Gemini.zip` e `IA_fisgraf-cli.exe`.
  - `crear_distribucion.bat` hace todo (manuales incluidos). Después se publica con `gh release create vX.Y.Z dist/* …`.
- **Versión:** está en `[workspace.package]` del `Cargo.toml` raíz, en `ia/claude/.claude-plugin/plugin.json`, en `.claude-plugin/marketplace.json` y en `ia/gemini/gemini-extension.json`.
- **Scripts `.bat`:** llaman a otros scripts con ruta completa (`call "%~dp0otro.bat"`). Claude Code define `NoDefaultCurrentDirectoryInExePath`, y entonces cmd no encuentra un script por su nombre solo. `.gitattributes` fuerza CRLF en `.bat` y `.ps1`.
- **Interfaz JSON de `fisgraf-cli`:** si cambia, actualiza la skill, el esquema de las herramientas en `mcp.rs`, `ia/guia_ia.md` y `AGENTS.md`.

## Reglas de trabajo
- **Al terminar un cambio:**
  - la compilación no debe dar errores;
  - `cargo test --workspace` debe quedar en verde;
  - clippy no debe dar advertencias nuevas.

  Después anótalo en `graficos/PLAN.md` y haz un commit.
- **Commits intermedios:** si la sesión puede cortarse a mitad de un cambio grande, haz commits intermedios con el prefijo `WIP:`.
- **Idioma:** interfaz, comentarios de usuario y mensajes en **español**; identificadores de código en inglés.
- **Gráficos:** todo texto que aparezca en el gráfico (etiquetas, leyendas, ecuaciones) se escribe en LaTeX.
- **Lógica numérica:** vive solo en `graficos/src`. FisErr y `fisgraf-cli` la usan; no la copies.

## Entorno
- **Sistema:** Windows 11, Rust 1.97 y MiKTeX 25.12, con `lualatex`, `pdflatex`, `latexmk`, `pdftoppm` y `pdftocairo` en el PATH.
- **Repositorio:** https://github.com/malvarezdam/FisGraf-FisErr (rama `main`). Las descargas van en sus Releases, no en git.
- **Barras invertidas en Rust:** al escribir código Rust con barras invertidas a través de un heredoc de bash, se pierden las dobles (`\\`). Usa la herramienta Write/Edit o cadenas crudas `r"…"`.
- **Perl con texto en español:** con `perl -pi`, usa `-CSD -Mutf8`. Sin `-Mutf8`, las tildes del propio script se corrompen.
- **Nombres de archivo en Windows:** no distinguen mayúsculas. `manual/MANUAL.pdf` y `manual/manual.pdf` son el mismo archivo; por eso los PDF publicados se llaman `Manual_*.pdf`.
