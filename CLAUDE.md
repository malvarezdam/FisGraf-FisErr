# FisGraf — instrucciones para Claude

Lee siempre `PLAN.md` antes de trabajar: allí están la arquitectura, las decisiones técnicas y las fases.

## Reglas de trabajo
- Haz **solo la fase pedida**. No adelantes trabajo de fases posteriores salvo lo mínimo para que compile.
- Al terminar una fase: `cargo build` sin errores, `cargo test` en verde y `cargo clippy` sin advertencias nuevas. Después marca la casilla en la sección "Progreso" de `PLAN.md` y haz un commit con el mensaje `Fase N: <resumen>`.
- Si la sesión puede cortarse a mitad de una fase, haz commits intermedios con el prefijo `WIP Fase N:`. Así la siguiente sesión retoma desde ahí.
- Si una decisión del plan resulta inviable (un crate, una API), elige la alternativa más cercana y anótalo en `PLAN.md` dentro de la fase correspondiente.
- Interfaz, comentarios de usuario y mensajes en **español**; identificadores de código en inglés.
- Todo texto que aparezca en el gráfico (etiquetas, leyendas, ecuaciones) se escribe en LaTeX.

## Entorno
- Windows 11, Rust 1.97, MiKTeX 25.12 (`lualatex`, `pdflatex`, `latexmk`, `pdftoppm`, `pdftocairo` en el PATH).
