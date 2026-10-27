//! Conversión del PDF a PNG (pdftoppm) y SVG (pdftocairo).

use std::path::Path;
use std::time::Duration;

use super::RenderError;
use super::compile::run;

const TIMEOUT: Duration = Duration::from_secs(60);

fn as_str(p: &Path) -> Result<&str, RenderError> {
    p.to_str().ok_or_else(|| RenderError::Io(format!("ruta no válida: {}", p.display())))
}

/// Escribe `png` (con extensión) a partir de la primera página de `pdf`.
pub fn pdf_to_png(pdf: &Path, png: &Path, dpi: u32) -> Result<(), RenderError> {
    let dir = pdf.parent().unwrap_or(Path::new("."));
    // pdftoppm añade la extensión: se le pasa la ruta sin ella.
    let prefix = png.with_extension("");
    let dpi = dpi.clamp(36, 2400).to_string();
    let ok = run("pdftoppm", &["-png", "-r", &dpi, "-singlefile", as_str(pdf)?, as_str(&prefix)?], dir, TIMEOUT)?;
    if ok && png.is_file() {
        Ok(())
    } else {
        Err(RenderError::Conversion("PNG".into()))
    }
}

pub fn pdf_to_svg(pdf: &Path, svg: &Path) -> Result<(), RenderError> {
    let dir = pdf.parent().unwrap_or(Path::new("."));
    let ok = run("pdftocairo", &["-svg", as_str(pdf)?, as_str(svg)?], dir, TIMEOUT)?;
    if ok && svg.is_file() {
        Ok(())
    } else {
        Err(RenderError::Conversion("SVG".into()))
    }
}
