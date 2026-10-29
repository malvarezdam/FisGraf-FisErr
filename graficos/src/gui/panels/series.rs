//! Pestaña Series: lista de mediciones y su configuración.

use egui::{RichText, Sense, Ui};

use crate::config::SeriesConfig;
use crate::gui::FisGrafApp;
use crate::gui::text::latex_to_plain;
use crate::gui::widgets::{self, ERROR_COLOR, MARKERS, combo, error_spec_edit, line_style_edit, range_edit, rgb_edit};

impl FisGrafApp {
    pub fn add_series(&mut self) -> u32 {
        let id = self.project.add_series();
        self.selected_series = Some(id);
        id
    }

    /// Elimina una serie y la quita de los ajustes que la usaban.
    pub fn remove_series(&mut self, id: u32) {
        let Some(pos) = self.project.series.iter().position(|s| s.id == id) else { return };
        self.project.series.remove(pos);
        for f in &mut self.project.fits {
            f.series_ids.retain(|s| *s != id);
        }
        self.selected_series = self.project.series.get(pos.min(self.project.series.len().saturating_sub(1))).map(|s| s.id);
    }

    pub fn duplicate_series(&mut self, id: u32) {
        let Some(pos) = self.project.series.iter().position(|s| s.id == id) else { return };
        let new_id = self.project.next_id();
        let n = self.project.series.len();
        let mut copy = self.project.series[pos].clone();
        copy.id = new_id;
        copy.label = format!("{} (copia)", copy.label);
        copy.color = crate::config::Rgb::palette(n);
        self.project.series.insert(pos + 1, copy);
        self.selected_series = Some(new_id);
    }

    pub fn move_series(&mut self, id: u32, up: bool) {
        let Some(pos) = self.project.series.iter().position(|s| s.id == id) else { return };
        let other = if up { pos.checked_sub(1) } else { Some(pos + 1).filter(|p| *p < self.project.series.len()) };
        if let Some(o) = other {
            self.project.series.swap(pos, o);
        }
    }

    /// Resumen de lo leído del Excel para una serie.
    fn series_status(&self, id: u32) -> Option<Result<String, String>> {
        let out = self.worker.latest.as_ref()?;
        Some(match out.computed.series.get(&id)? {
            Ok(d) => {
                let mut s = format!("{} puntos", d.len());
                if !d.skipped.is_empty() {
                    s += &format!(" ({} filas vacías descartadas)", d.skipped.len());
                }
                Ok(s)
            }
            Err(e) => Err(e.clone()),
        })
    }

    pub(crate) fn series_panel(&mut self, ui: &mut Ui) {
        ui.heading("Series de mediciones");
        ui.weak("Cada serie es un conjunto de puntos con su propia leyenda y estilo.");
        let sel = self.selected_series.filter(|id| self.project.series_by_id(*id).is_some());
        ui.horizontal(|ui| {
            if ui.button("+ Nueva serie").clicked() {
                self.add_series();
            }
            if let Some(id) = sel {
                if ui.button("Duplicar").clicked() {
                    self.duplicate_series(id);
                }
                if ui.button("Eliminar").clicked() {
                    self.remove_series(id);
                }
                if ui.button("↑").on_hover_text("Subir").clicked() {
                    self.move_series(id, true);
                }
                if ui.button("↓").on_hover_text("Bajar").clicked() {
                    self.move_series(id, false);
                }
            }
        });
        ui.add_space(4.0);

        if self.project.series.is_empty() {
            ui.label("No hay series. Créala aquí o selecciona columnas en la pestaña Datos.");
            return;
        }
        let statuses: Vec<_> = self.project.series.iter().map(|s| self.series_status(s.id)).collect();
        for (s, status) in self.project.series.iter().zip(&statuses) {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
                ui.painter().rect_filled(rect, 2.0, widgets::to_color32(s.color));
                let name = latex_to_plain(&s.label);
                let text = if s.visible { RichText::new(name) } else { RichText::new(name).weak() };
                if ui.selectable_label(sel == Some(s.id), text).clicked() {
                    self.selected_series = Some(s.id);
                }
                match status {
                    Some(Ok(t)) => {
                        ui.weak(t);
                    }
                    Some(Err(_)) => {
                        ui.colored_label(ERROR_COLOR, "⚠ error");
                    }
                    None => {}
                }
            });
        }
        ui.separator();

        let Some(id) = self.selected_series else { return };
        let status = self.series_status(id);
        let Some(s) = self.project.series.iter_mut().find(|s| s.id == id) else { return };
        series_editor(ui, s);
        match status {
            Some(Ok(t)) => {
                ui.colored_label(widgets::OK_COLOR, format!("✔ {t}"));
            }
            Some(Err(e)) => {
                ui.colored_label(ERROR_COLOR, e);
            }
            None => {}
        }
    }
}

fn series_editor(ui: &mut Ui, s: &mut SeriesConfig) {
    let id = s.id;
    egui::Grid::new(("serie", id)).num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Leyenda (LaTeX)");
        ui.vertical(|ui| {
            ui.add(egui::TextEdit::singleline(&mut s.label).desired_width(230.0));
            ui.weak(format!("Vista: {}", latex_to_plain(&s.label)));
        });
        ui.end_row();

        ui.label("");
        ui.horizontal(|ui| {
            ui.checkbox(&mut s.visible, "Visible");
            ui.checkbox(&mut s.show_in_legend, "En la leyenda");
        });
        ui.end_row();

        ui.label("Rango x");
        range_edit(ui, &mut s.x_range, "Hoja1!A2:A20");
        ui.end_row();
        ui.label("Rango y");
        range_edit(ui, &mut s.y_range, "Hoja1!B2:B20");
        ui.end_row();
        ui.label("Error de x");
        error_spec_edit(ui, (id, "ex"), &mut s.x_error);
        ui.end_row();
        ui.label("Error de y");
        error_spec_edit(ui, (id, "ey"), &mut s.y_error);
        ui.end_row();

        ui.label("Color");
        rgb_edit(ui, &mut s.color);
        ui.end_row();
        ui.label("Marcador");
        ui.horizontal(|ui| {
            combo(ui, (id, "marcador"), "Marcador", &mut s.marker, &MARKERS);
            ui.add(egui::DragValue::new(&mut s.marker_size_pt).speed(0.05).range(0.5..=8.0).suffix(" pt"));
        });
        ui.end_row();
        ui.label("Topes de error");
        ui.add(egui::DragValue::new(&mut s.error_cap_pt).speed(0.05).range(0.0..=6.0).suffix(" pt"))
            .on_hover_text("0 = barras sin tope");
        ui.end_row();

        ui.label("Unir puntos");
        let mut connect = s.connect_points.is_some();
        if ui.checkbox(&mut connect, "Con una línea").changed() {
            s.connect_points = connect.then(|| crate::config::LineStyle { color: None, ..Default::default() });
        }
        ui.end_row();
    });
    if let Some(line) = &mut s.connect_points {
        ui.indent(("linea", id), |ui| line_style_edit(ui, (id, "union"), line, Some(s.color)));
    }
}
