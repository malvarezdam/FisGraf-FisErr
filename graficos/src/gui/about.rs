//! Ventana «Acerca de», común a FisGraf y FisErr: autoría, licencia y cómo citar.

use egui::{RichText, Ui};

pub const AUTHOR: &str = "Martín Álvarez de Araya";
pub const REPOSITORY: &str = "https://github.com/malvarezdam/FisGraf-FisErr";

/// Referencia lista para copiar en un informe.
pub fn citation() -> String {
    format!("Álvarez de Araya, M. (2026). FisGraf y FisErr (v{}) [Software]. {REPOSITORY}", env!("CARGO_PKG_VERSION"))
}

/// Muestra la ventana mientras `open` sea verdadero.
pub fn show(ui: &mut Ui, open: &mut bool, app_name: &str, summary: &str) {
    if !*open {
        return;
    }
    let modal = egui::Modal::new(egui::Id::unique("acerca_de")).show(ui.ctx(), |ui| {
        ui.set_max_width(460.0);
        ui.heading(format!("{app_name} {}", env!("CARGO_PKG_VERSION")));
        ui.label(summary);
        ui.add_space(6.0);
        ui.label(RichText::new(format!("© 2026 {AUTHOR}. Todos los derechos reservados.")).strong());
        ui.label("Uso gratuito para cualquier fin, citando siempre al autor. Está prohibido venderlo, cobrar por él, presentarlo como propio o usarlo para entrenar inteligencia artificial.");
        ui.add_space(6.0);
        ui.label(RichText::new("Cómo citarlo").strong());
        let cite = citation();
        ui.label(RichText::new(&cite).monospace());
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("Copiar cita").clicked() {
                ui.ctx().copy_text(cite.clone());
            }
            ui.hyperlink_to("Repositorio y licencia", REPOSITORY);
        });
        ui.add_space(6.0);
        if ui.button("Cerrar").clicked() {
            *open = false;
        }
    });
    if modal.should_close() {
        *open = false;
    }
}
