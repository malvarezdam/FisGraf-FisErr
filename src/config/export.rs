//! Formatos y destino de exportación.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportConfig {
    /// Carpeta de salida. `None` = junto al archivo del proyecto.
    pub output_dir: Option<PathBuf>,
    /// Nombre base de los archivos, sin extensión.
    pub file_stem: String,
    pub pdf: bool,
    pub png: bool,
    pub png_dpi: u32,
    pub svg: bool,
    /// Documento `.tex` completo y compilable.
    pub tex_standalone: bool,
    /// Solo el entorno `tikzpicture`, para `\input{}` en un informe.
    pub tex_snippet: bool,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            output_dir: None,
            file_stem: "grafico".into(),
            pdf: true,
            png: false,
            png_dpi: 300,
            svg: false,
            tex_standalone: false,
            tex_snippet: false,
        }
    }
}
