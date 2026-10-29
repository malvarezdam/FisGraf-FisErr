//! Pestaña Referencias: curvas teóricas fijas para comparar con los datos.

use egui::{RichText, Ui};

use crate::config::{NamedValue, ReferenceFunction};
use crate::gui::FisGrafApp;
use crate::gui::text::latex_to_plain;
use crate::gui::widgets::{ERROR_COLOR, WARN_COLOR, line_style_edit, number};
use crate::model::{LatexOptions, Model, suggest_parameters};

impl FisGrafApp {
    pub fn add_reference(&mut self) -> u32 {
        let id = self.project.next_id();
        self.project.references.push(ReferenceFunction { id, expression: "x".into(), ..Default::default() });
        self.selected_reference = Some(id);
        id
    }

    pub fn remove_reference(&mut self, id: u32) {
        let Some(pos) = self.project.references.iter().position(|r| r.id == id) else { return };
        self.project.references.remove(pos);
        self.selected_reference =
            self.project.references.get(pos.min(self.project.references.len().saturating_sub(1))).map(|r| r.id);
    }

    pub(crate) fn references_panel(&mut self, ui: &mut Ui) {
        if self.selected_reference.is_none_or(|id| !self.project.references.iter().any(|r| r.id == id)) {
            self.selected_reference = self.project.references.first().map(|r| r.id);
        }
        ui.heading("Funciones de referencia");
        ui.weak("Curvas teóricas fijas (sin ajuste) para comparar con las mediciones.");
        let sel = self.selected_reference.filter(|id| self.project.references.iter().any(|r| r.id == *id));
        ui.horizontal(|ui| {
            if ui.button("+ Nueva referencia").clicked() {
                self.add_reference();
            }
            if let Some(id) = sel {
                if ui.button("Duplicar").clicked() {
                    let pos = self.project.references.iter().position(|r| r.id == id).expect("existe");
                    let mut copy = self.project.references[pos].clone();
                    copy.id = self.project.next_id();
                    self.selected_reference = Some(copy.id);
                    self.project.references.insert(pos + 1, copy);
                }
                if ui.button("Eliminar").clicked() {
                    self.remove_reference(id);
                }
            }
        });
        if self.project.references.is_empty() {
            ui.label("No hay funciones de referencia.");
            return;
        }
        for r in &mut self.project.references {
            ui.horizontal(|ui| {
                ui.checkbox(&mut r.enabled, "").on_hover_text("Visible");
                let name = if r.label.trim().is_empty() { r.expression.clone() } else { latex_to_plain(&r.label) };
                if ui.selectable_label(sel == Some(r.id), name).clicked() {
                    self.selected_reference = Some(r.id);
                }
            });
        }
        ui.separator();
        let comma = crate::pipeline::decimal_comma(&self.project);
        let Some(id) = self.selected_reference else { return };
        let Some(r) = self.project.references.iter_mut().find(|r| r.id == id) else { return };
        reference_editor(ui, r, comma);
    }
}

fn reference_editor(ui: &mut Ui, r: &mut ReferenceFunction, comma: bool) {
    let id = r.id;
    egui::Grid::new(("referencia", id)).num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Leyenda (LaTeX)");
        ui.vertical(|ui| {
            ui.add(egui::TextEdit::singleline(&mut r.label).hint_text("vacío = la ecuación").desired_width(240.0));
            ui.checkbox(&mut r.show_in_legend, "En la leyenda");
        });
        ui.end_row();
        ui.label("Ecuación");
        ui.add(egui::TextEdit::singleline(&mut r.expression).font(egui::TextStyle::Monospace).desired_width(260.0));
        ui.end_row();
        ui.label("Variable");
        ui.add(egui::TextEdit::singleline(&mut r.variable).desired_width(80.0));
        ui.end_row();
        ui.label("LaTeX propio");
        ui.add(egui::TextEdit::singleline(&mut r.latex_override).hint_text("vacío = automático").desired_width(260.0));
        ui.end_row();
    });

    ui.add_space(4.0);
    ui.strong("Constantes");
    let mut remove = None;
    egui::Grid::new(("constantes", id)).spacing([8.0, 4.0]).show(ui, |ui| {
        for (k, c) in r.constants.iter_mut().enumerate() {
            ui.add(egui::TextEdit::singleline(&mut c.name).desired_width(70.0));
            ui.label("=");
            number(ui, &mut c.value);
            if ui.small_button("✖").on_hover_text("Quitar").clicked() {
                remove = Some(k);
            }
            ui.end_row();
        }
    });
    if let Some(k) = remove {
        r.constants.remove(k);
    }
    let known: Vec<String> = r.constants.iter().map(|c| c.name.clone()).collect();
    ui.horizontal(|ui| {
        if ui.button("+ Constante").clicked() {
            r.constants.push(NamedValue { name: format!("c{}", r.constants.len() + 1), value: 1.0 });
        }
        if let Ok(missing) = suggest_parameters(&r.expression, &r.variable, &known)
            && !missing.is_empty()
        {
            ui.colored_label(WARN_COLOR, format!("Sin valor: {}", missing.join(", ")));
            if ui.button("Agregarlas").clicked() {
                r.constants.extend(missing.into_iter().map(|name| NamedValue { name, value: 1.0 }));
            }
        }
    });

    let consts: Vec<(String, f64)> = r.constants.iter().map(|c| (c.name.clone(), c.value)).collect();
    match Model::new(&r.expression, &r.variable, &[], &consts) {
        Ok(m) => {
            let latex = if r.latex_override.trim().is_empty() {
                m.to_latex(&LatexOptions { decimal_comma: comma, ..Default::default() })
            } else {
                r.latex_override.clone()
            };
            ui.label(RichText::new(format!("f = {}", latex_to_plain(&latex))).size(15.0));
        }
        Err(e) => {
            ui.colored_label(ERROR_COLOR, e.to_string());
        }
    }

    ui.add_space(6.0);
    ui.strong("Curva");
    line_style_edit(ui, (id, "linea"), &mut r.line, None);
    ui.horizontal(|ui| {
        let mut manual = r.domain.is_some();
        if ui.checkbox(&mut manual, "Dominio manual").on_hover_text("Por defecto cubre todo el eje x").changed() {
            r.domain = manual.then_some((0.0, 1.0));
        }
        if let Some((a, b)) = &mut r.domain {
            number(ui, a);
            ui.label("a");
            number(ui, b);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Puntos de la curva");
        ui.add(egui::DragValue::new(&mut r.samples).range(2..=5000));
    });
}
