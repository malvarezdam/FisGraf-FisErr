// En la versión compilada no se abre una consola junto a la ventana.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("FisGraf")
            .with_inner_size([1400.0, 860.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    // `fisgraf proyecto.fisgraf.json` abre ese proyecto.
    let project = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    eframe::run_native(
        "FisGraf",
        options,
        Box::new(move |_cc| {
            let mut app = fisgraf::gui::FisGrafApp::default();
            if let Some(p) = project {
                app.open_path(&p);
            }
            Ok(Box::new(app))
        }),
    )
}
