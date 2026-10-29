//! Pestaña Propagación: lista de cálculos de error del proyecto. La
//! calculadora en sí está en [`crate::gui::calculator`] (la comparte FisErr).

use egui::Ui;

use crate::config::PropagationConfig;
use crate::gui::FisGrafApp;

impl FisGrafApp {
    pub fn add_propagation(&mut self) -> u32 {
        let id = self.project.next_id();
        self.project.propagations.push(PropagationConfig { id, ..Default::default() });
        self.selected_propagation = Some(id);
        id
    }

    pub(crate) fn propagation_panel(&mut self, ui: &mut Ui) {
        ui.heading("Propagación de errores");
        ui.weak("Calcula el error de una magnitud a partir de mediciones con su incertidumbre, sin necesidad de gráfico.");
        if self.selected_propagation.is_none_or(|id| !self.project.propagations.iter().any(|p| p.id == id)) {
            self.selected_propagation = self.project.propagations.first().map(|p| p.id);
        }
        let sel = self.selected_propagation;
        ui.horizontal(|ui| {
            if ui.button("+ Nuevo cálculo").clicked() {
                self.add_propagation();
            }
            if let Some(id) = sel {
                if ui.button("Duplicar").clicked() {
                    let pos = self.project.propagations.iter().position(|p| p.id == id).expect("existe");
                    let mut copy = self.project.propagations[pos].clone();
                    copy.id = self.project.next_id();
                    self.selected_propagation = Some(copy.id);
                    self.project.propagations.insert(pos + 1, copy);
                }
                if ui.button("Eliminar").clicked() {
                    self.project.propagations.retain(|p| p.id != id);
                    self.selected_propagation = None;
                }
            }
        });
        if self.project.propagations.is_empty() {
            ui.label("No hay cálculos. Crea uno con «+ Nuevo cálculo».");
            return;
        }
        for p in &self.project.propagations {
            let text = format!("{} = {}", p.name, if p.expression.is_empty() { "…" } else { &p.expression });
            if ui.selectable_label(sel == Some(p.id), text).clicked() {
                self.selected_propagation = Some(p.id);
            }
        }
        ui.separator();

        let comma = crate::pipeline::decimal_comma(&self.project);
        let Some(id) = self.selected_propagation else { return };
        let Some(cfg) = self.project.propagations.iter_mut().find(|p| p.id == id) else { return };
        self.calculator.ui(ui, cfg, comma);
    }
}
