# Análisis de datos (FisGraf y FisErr): instrucciones para Claude

Workspace de Cargo con dos aplicaciones que comparten la lógica numérica:

| Carpeta | Paquete | Qué es |
|---|---|---|
| `graficos/` | `fisgraf` | **FisGraf**: gráficos de datos con ajustes, incertidumbres y LaTeX. Contiene la biblioteca con toda la lógica (`src/lib.rs`) y la herramienta de línea de comandos **`fisgraf-cli`** (`src/bin/fisgraf-cli/`: `main.rs` con los comandos JSON y `mcp.rs` con el servidor MCP). Su arquitectura y su historial están en `graficos/PLAN.md`. |
| `errores/` | `fiserr` | **FisErr**: calculadora de propagación de errores. Reutiliza `fisgraf::propagation` y `fisgraf::gui::calculator`; no duplica lógica. |

Repositorio público: https://github.com/malvarezdam/FisGraf-FisErr (rama `main`). Las descargas (portables, paquete para Claude y `fisgraf-cli.exe`) van en sus Releases, no en git.

**Acceso para IA** (todo usa `fisgraf-cli`):
- **Skill** `plugin/skills/fisgraf/` (SKILL.md, referencia.md y `ejemplos/`). Es la única copia en git y sirve para tres cosas:
  - el plugin de Claude Code (`plugin/.claude-plugin/plugin.json`, publicado por `.claude-plugin/marketplace.json`);
  - el contenido de `FisGraf-para-Claude.zip`;
  - la copia instalada en `~/.claude/skills/fisgraf/`, que lleva además el `.exe`. Si cambias la skill, actualiza también esta copia.

  La skill no debe tener rutas de este computador. Busca el `.exe` en este orden: junto al SKILL.md, en `~/.fisgraf/`, en el PATH; si no lo encuentra, lo descarga de la última Release.
- **Servidor MCP**: `fisgraf-cli mcp` (stdio). `instalar.ps1` lo descarga y lo registra en Claude Desktop; ese script debe quedar solo con caracteres ASCII.
- **`AGENTS.md`**: resume el uso para otros agentes.

## Comandos
- Compilar todo: `cargo build --workspace`. Probar todo: `cargo test --workspace`. Revisar: `cargo clippy --workspace --all-targets`.
- Abrir las aplicaciones: `cargo run -p fisgraf` y `cargo run -p fiserr`.
- Ejecutables: `graficos/crear_ejecutable.bat` y `errores/crear_ejecutable.bat` copian `FisGraf.exe` y `FisErr.exe` a sus carpetas. Están ignorados en git.
- Versión portable: `crear_portable.bat` arma `portable/`, `dist/FisGraf-portable.zip` y `dist/FisErr-portable.zip`.
  - Compila FisGraf y FisErr con `+crt-static` en `target/portable` e incluye los ejemplos y los manuales.
  - No incluye `fisgraf-cli` (decisión del usuario).
- Manuales: `manual/compilar.bat` → `MANUAL.pdf`, `MANUAL_FisErr.pdf` y `manual/manual_portable.pdf`.
- Paquete para Claude: `crear_skill_portable.bat` → `portable_claude/`, `dist/FisGraf-para-Claude.zip` y `dist/fisgraf-cli.exe`.
- Release:
  - `crear_distribucion.bat` crea todo `dist/`; después se publica con `gh release create vX.Y.Z dist/* …`.
  - Al cambiar la versión, actualízala en `graficos/Cargo.toml`, `plugin/.claude-plugin/plugin.json` y `.claude-plugin/marketplace.json`.
- Los `.bat` llaman a otros scripts con ruta completa (`call "%~dp0otro.bat"`). Claude Code define `NoDefaultCurrentDirectoryInExePath`, y con eso cmd no encuentra un script por su nombre solo. `.gitattributes` fuerza CRLF en `.bat` y `.ps1`.
- La herramienta queda en `target/release/fisgraf-cli.exe` (`cargo build --release -p fisgraf --bin fisgraf-cli`). Si cambia su interfaz JSON, actualiza la skill, el esquema de las herramientas en `mcp.rs` y `AGENTS.md`.

## Reglas de trabajo
- Al terminar un cambio: compilación sin errores, `cargo test --workspace` en verde y clippy sin advertencias nuevas. Después anótalo en `graficos/PLAN.md` y haz un commit.
- Si la sesión puede cortarse a mitad de un cambio grande, haz commits intermedios con el prefijo `WIP:`.
- Interfaz, comentarios de usuario y mensajes en **español**; identificadores de código en inglés.
- Todo texto que aparezca en el gráfico (etiquetas, leyendas, ecuaciones) se escribe en LaTeX.
- La lógica numérica vive solo en `graficos/src`. FisErr y `fisgraf-cli` la usan; no la copies.

## Entorno
- Windows 11, Rust 1.97, MiKTeX 25.12 (`lualatex`, `pdflatex`, `latexmk`, `pdftoppm`, `pdftocairo` en el PATH).
- Al escribir código Rust con barras invertidas a través de un heredoc de bash, se pierden las dobles (`\\`). Usa la herramienta Write/Edit o cadenas crudas `r"…"`.
- Con `perl -pi` sobre texto en español, usa `-CSD -Mutf8`. Sin `-Mutf8`, las tildes del propio script se corrompen.
