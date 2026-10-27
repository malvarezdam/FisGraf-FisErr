//! Interfaz gráfica (egui). Los paneles completos llegan en las Fases 6–8.

use crate::config::ProjectConfig;

#[derive(Default)]
pub struct FisGrafApp {
    pub project: ProjectConfig,
}

impl eframe::App for FisGrafApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default_margins().show(ui, |ui| {
            ui.heading("FisGraf");
            ui.label("Gráficos de datos de física con ajuste de modelos y LaTeX.");
            ui.separator();
            ui.label(format!(
                "Proyecto: {} series, {} ajustes, {} funciones de referencia.",
                self.project.series.len(),
                self.project.fits.len(),
                self.project.references.len()
            ));
            ui.weak("Interfaz en construcción (Fases 6–8 del plan).");
        });
    }
}
