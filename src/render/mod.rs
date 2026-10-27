//! Generación del gráfico en LaTeX (pgfplots), compilación y conversión
//! a PNG/SVG.
//!
//! Flujo: [`crate::pipeline::build_scene`] → [`pgfplots::generate`] →
//! [`compile::compile_pdf`] → [`convert`]. [`export::export`] hace todo el
//! recorrido según la configuración de exportación.

pub mod compile;
pub mod convert;
pub mod export;
pub mod pgfplots;
pub mod scene;

pub use export::{ExportReport, export};
pub use pgfplots::{TexDocument, generate};
pub use scene::Scene;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RenderError {
    #[error("no se encontró «{0}». Instala MiKTeX (https://miktex.org) o TeX Live y reinicia FisGraf")]
    MissingProgram(String),

    #[error("{}{message}{}{}",
        line.map_or(String::new(), |l| format!("error de LaTeX en la línea {l}: ")),
        source_line.as_ref().map_or(String::new(), |s| format!("\n  → {s}")),
        if context.is_empty() { String::new() } else { format!("\n{context}") })]
    Latex {
        line: Option<usize>,
        message: String,
        /// Línea del `.tex` donde ocurrió el error.
        source_line: Option<String>,
        context: String,
    },

    #[error("{program} no terminó en {seconds} s (¿MiKTeX está esperando instalar un paquete?)")]
    Timeout { program: String, seconds: u64 },

    #[error("no se pudo convertir el PDF a {0}")]
    Conversion(String),

    #[error("error de archivos: {0}")]
    Io(String),
}
