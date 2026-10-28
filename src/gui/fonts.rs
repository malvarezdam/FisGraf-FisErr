//! Fuentes de respaldo del sistema: la fuente de egui no trae letras griegas,
//! flechas ni símbolos como σ, χ, ✔ o →, que la interfaz usa.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};

/// Candidatos por sistema operativo, en orden de prioridad.
const CANDIDATES: [&str; 6] = [
    "C:/Windows/Fonts/segoeui.ttf",
    "C:/Windows/Fonts/seguisym.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
    "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    "/Library/Fonts/Arial Unicode.ttf",
];

/// Añade las fuentes del sistema como respaldo. Devuelve cuántas cargó.
pub fn install(ctx: &egui::Context) -> usize {
    let mut defs = FontDefinitions::default();
    let mut loaded = 0;
    for path in CANDIDATES {
        let Ok(bytes) = std::fs::read(path) else { continue };
        let name = format!("sistema-{loaded}");
        defs.font_data.insert(name.clone(), Arc::new(FontData::from_owned(bytes)));
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            defs.families.entry(family).or_default().push(name.clone());
        }
        loaded += 1;
    }
    if loaded > 0 {
        ctx.set_fonts(defs);
    }
    loaded
}
