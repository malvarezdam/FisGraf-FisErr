// En la versión compilada no se abre una consola junto a la ventana.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

/// Ícono de la ventana (incluido en el ejecutable).
fn icon() -> Option<Arc<egui::IconData>> {
    let img = image::load_from_memory(include_bytes!("../assets/logo.png")).ok()?.to_rgba8();
    let (width, height) = img.dimensions();
    Some(Arc::new(egui::IconData { rgba: img.into_raw(), width, height }))
}

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("FisErr")
        .with_inner_size([1250.0, 820.0])
        .with_min_inner_size([800.0, 520.0]);
    if let Some(icon) = icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    // `fiserr archivo.fiserr.json` abre ese archivo.
    let file = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    eframe::run_native(
        "FisErr",
        options,
        Box::new(move |_cc| {
            let mut app = fiserr::FisErrApp::default();
            if let Some(p) = file {
                app.open_path(&p);
            }
            Ok(Box::new(app))
        }),
    )
}
