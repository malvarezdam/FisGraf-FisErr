# Análisis de datos (FisGraf y FisErr): instrucciones para Claude

Workspace de Cargo con dos aplicaciones que comparten la lógica numérica:

| Carpeta | Paquete | Qué es |
|---|---|---|
| `graficos/` | `fisgraf` | **FisGraf**: gráficos de datos con ajustes, incertidumbres y LaTeX. Contiene la biblioteca con toda la lógica (`src/lib.rs`) y la herramienta de línea de comandos **`fisgraf-cli`** (`src/bin/fisgraf-cli.rs`). Su arquitectura y su historial están en `graficos/PLAN.md`. |
| `errores/` | `fiserr` | **FisErr**: calculadora de propagación de errores. Reutiliza `fisgraf::propagation` y `fisgraf::gui::calculator`; no duplica lógica. |

La skill `.claude/skills/fisgraf/` explica cómo usar `fisgraf-cli` como herramienta en tareas de análisis de datos. Hay una copia instalada en `~/.claude/skills/fisgraf/`: si cambias la skill, actualiza ambas.

## Comandos
- Compilar todo: `cargo build --workspace`. Probar todo: `cargo test --workspace`. Revisar: `cargo clippy --workspace --all-targets`.
- Abrir las aplicaciones: `cargo run -p fisgraf` y `cargo run -p fiserr`.
- Ejecutables: `graficos/crear_ejecutable.bat` y `errores/crear_ejecutable.bat` copian `FisGraf.exe` y `FisErr.exe` a sus carpetas. Están ignorados en git.
- Versión portable: `crear_portable.bat` arma `portable/` (ignorada en git) con FisGraf y FisErr compilados con `+crt-static` en `target/portable`, ejemplos y manuales; sin `fisgraf-cli` (decisión del usuario). Manuales: `manual/compilar.bat` → `MANUAL.pdf` y `MANUAL_FisErr.pdf`.
- La herramienta queda en `target/release/fisgraf-cli.exe` (`cargo build --release -p fisgraf --bin fisgraf-cli`). Si cambia su interfaz JSON, actualiza la skill.

## Reglas de trabajo
- Al terminar un cambio: compilación sin errores, `cargo test --workspace` en verde y clippy sin advertencias nuevas. Después anótalo en `graficos/PLAN.md` y haz un commit.
- Si la sesión puede cortarse a mitad de un cambio grande, haz commits intermedios con el prefijo `WIP:`.
- Interfaz, comentarios de usuario y mensajes en **español**; identificadores de código en inglés.
- Todo texto que aparezca en el gráfico (etiquetas, leyendas, ecuaciones) se escribe en LaTeX.
- La lógica numérica vive solo en `graficos/src`. FisErr y `fisgraf-cli` la usan; no la copies.

## Entorno
- Windows 11, Rust 1.97, MiKTeX 25.12 (`lualatex`, `pdflatex`, `latexmk`, `pdftoppm`, `pdftocairo` en el PATH).
- Al escribir código Rust con barras invertidas a través de un heredoc de bash, se pierden las dobles (`\\`). Usa la herramienta Write/Edit o cadenas crudas `r"…"`.
