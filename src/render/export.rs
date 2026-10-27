//! Exportación a PDF, PNG, SVG, `.tex` completo y fragmento `tikzpicture`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::ProjectConfig;

use super::RenderError;
use super::compile::compile_pdf;
use super::convert::{pdf_to_png, pdf_to_svg};
use super::pgfplots::{TexDocument, generate};
use super::scene::Scene;

/// Tiempo máximo de compilación. Generoso porque MiKTeX puede instalar
/// paquetes o reconstruir la caché de fuentes la primera vez.
pub const COMPILE_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug, Clone, Default)]
pub struct ExportReport {
    pub files: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

/// Carpeta temporal de compilación, propia de este proceso.
pub fn build_dir() -> PathBuf {
    std::env::temp_dir().join(format!("fisgraf-{}", std::process::id()))
}

/// Encabezado del fragmento: qué debe tener el preámbulo del informe.
fn snippet(doc: &TexDocument) -> String {
    let needs: String = doc.preamble.lines().map(|l| format!("%   {l}\n")).collect();
    format!("% Generado por FisGraf. Uso: \\input{{este_archivo}} dentro de un documento con:\n{needs}{}", doc.picture)
}

pub fn export(project: &ProjectConfig, scene: &Scene, out_dir: &Path) -> Result<ExportReport, RenderError> {
    let cfg = &project.export;
    let stem = if cfg.file_stem.trim().is_empty() { "grafico" } else { cfg.file_stem.trim() };
    std::fs::create_dir_all(out_dir).map_err(|e| RenderError::Io(e.to_string()))?;
    let doc = generate(project, scene);
    let mut report = ExportReport { warnings: doc.warnings.clone(), ..Default::default() };
    let write = |path: PathBuf, text: &str, report: &mut ExportReport| -> Result<(), RenderError> {
        std::fs::write(&path, text).map_err(|e| RenderError::Io(format!("{}: {e}", path.display())))?;
        report.files.push(path);
        Ok(())
    };

    if cfg.tex_standalone {
        write(out_dir.join(format!("{stem}.tex")), &doc.full, &mut report)?;
    }
    if cfg.tex_snippet {
        write(out_dir.join(format!("{stem}_tikz.tex")), &snippet(&doc), &mut report)?;
    }
    if cfg.pdf || cfg.png || cfg.svg {
        let pdf = compile_pdf(&doc.full, project.figure.engine, &build_dir(), stem, COMPILE_TIMEOUT)?;
        if cfg.pdf {
            let dest = out_dir.join(format!("{stem}.pdf"));
            std::fs::copy(&pdf, &dest).map_err(|e| RenderError::Io(format!("{}: {e}", dest.display())))?;
            report.files.push(dest);
        }
        if cfg.png {
            let dest = out_dir.join(format!("{stem}.png"));
            pdf_to_png(&pdf, &dest, cfg.png_dpi)?;
            report.files.push(dest);
        }
        if cfg.svg {
            let dest = out_dir.join(format!("{stem}.svg"));
            pdf_to_svg(&pdf, &dest)?;
            report.files.push(dest);
        }
    }
    Ok(report)
}
