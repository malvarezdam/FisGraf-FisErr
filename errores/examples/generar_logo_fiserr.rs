//! Genera `errores/assets/logo.png` (512 px, fondo transparente) y `errores/assets/fiserr.ico`
//! (16 a 256 px) a partir de `errores/assets/logo.tex`.
//!
//! Uso: `cargo run -p fiserr --example generar_logo_fiserr`

use std::path::Path;
use std::process::Command;

use fisgraf::config::LatexEngine;
use fisgraf::render::compile::compile_pdf;
use fisgraf::render::export::COMPILE_TIMEOUT;
use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::imageops::FilterType;
use image::{ExtendedColorType, ImageEncoder};

fn main() -> anyhow::Result<()> {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let tex = std::fs::read_to_string(assets.join("logo.tex"))?;
    let build = std::env::temp_dir().join("fiserr-logo");
    let pdf = compile_pdf(&tex, LatexEngine::PdfLatex, &build, "logo", COMPILE_TIMEOUT)?;

    // pdftocairo conserva la transparencia de las esquinas redondeadas.
    let prefix = build.join("logo_png");
    let ok = Command::new("pdftocairo")
        .args(["-png", "-transp", "-singlefile", "-scale-to", "1024"])
        .arg(&pdf)
        .arg(&prefix)
        .status()?
        .success();
    anyhow::ensure!(ok, "pdftocairo falló");
    let big = image::open(prefix.with_extension("png"))?.to_rgba8();

    let png = image::imageops::resize(&big, 512, 512, FilterType::Lanczos3);
    png.save(assets.join("logo.png"))?;

    let mut frames = Vec::new();
    for size in [16u32, 24, 32, 48, 64, 128, 256] {
        let img = image::imageops::resize(&big, size, size, FilterType::Lanczos3);
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes).write_image(img.as_raw(), size, size, ExtendedColorType::Rgba8)?;
        frames.push(IcoFrame::with_encoded(bytes, size, size, ExtendedColorType::Rgba8)?);
    }
    let file = std::fs::File::create(assets.join("fiserr.ico"))?;
    IcoEncoder::new(file).encode_images(&frames)?;
    println!("Escritos errores/assets/logo.png y errores/assets/fiserr.ico");
    Ok(())
}
