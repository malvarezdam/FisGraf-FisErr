//! Pestaña Ejes: título, límites, escala, marcas y formato numérico de x e y.

use egui::Ui;

use crate::config::{AxisConfig, TickSpacing};
use crate::data::parse_number;
use crate::gui::FisGrafApp;
use crate::gui::names::{NOTATIONS, SCALES, TICK_DIRECTIONS};
use crate::gui::text::latex_to_plain;
use crate::gui::widgets::{ERROR_COLOR, combo, number, optional_number};

/// Interpreta una lista de posiciones: `1; 2,5; 4` o `1 2.5 4`.
pub fn parse_tick_list(text: &str) -> Option<Vec<f64>> {
    let sep = if text.contains(';') { ';' } else { ' ' };
    let v: Option<Vec<f64>> = text.split(sep).map(str::trim).filter(|s| !s.is_empty()).map(parse_number).collect();
    v.filter(|v| !v.is_empty())
}

impl FisGrafApp {
    pub(crate) fn axes_panel(&mut self, ui: &mut Ui) {
        ui.heading("Ejes");
        egui::CollapsingHeader::new("Eje x (variable independiente)")
            .default_open(true)
            .show(ui, |ui| axis_editor(ui, "x", &mut self.project.x_axis));
        egui::CollapsingHeader::new("Eje y (variable dependiente)")
            .default_open(true)
            .show(ui, |ui| axis_editor(ui, "y", &mut self.project.y_axis));
        ui.weak("La coma decimal del eje y se usa también en ecuaciones, unidades y el cuadro de resultados.");
    }
}

fn axis_editor(ui: &mut Ui, id: &str, a: &mut AxisConfig) {
    egui::Grid::new(("eje", id)).num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Título (LaTeX)");
        ui.vertical(|ui| {
            ui.add(egui::TextEdit::singleline(&mut a.label).hint_text("Periodo $T$ [\\unit{s}]").desired_width(250.0));
            ui.weak(format!("Vista: {}", latex_to_plain(&a.label)));
        });
        ui.end_row();

        ui.label("Límites");
        ui.vertical(|ui| {
            ui.horizontal(|ui| optional_number(ui, "mínimo manual", &mut a.min, 0.0));
            ui.horizontal(|ui| optional_number(ui, "máximo manual", &mut a.max, 1.0));
        });
        ui.end_row();

        ui.label("Escala");
        combo(ui, (id, "escala"), "Escala", &mut a.scale, &SCALES);
        ui.end_row();

        ui.label("Marcas");
        ui.vertical(|ui| {
            let kind = match a.ticks.spacing {
                TickSpacing::Auto => 0,
                TickSpacing::Distance(_) => 1,
                TickSpacing::Manual(_) => 2,
            };
            let mut k = kind;
            ui.horizontal(|ui| {
                ui.radio_value(&mut k, 0, "Automáticas");
                ui.radio_value(&mut k, 1, "Cada…");
                ui.radio_value(&mut k, 2, "En posiciones");
            });
            if k != kind {
                a.ticks.spacing = match k {
                    1 => TickSpacing::Distance(1.0),
                    2 => TickSpacing::Manual(vec![0.0, 1.0]),
                    _ => TickSpacing::Auto,
                };
            }
            match &mut a.ticks.spacing {
                TickSpacing::Auto => {}
                TickSpacing::Distance(d) => {
                    ui.horizontal(|ui| {
                        ui.label("Distancia");
                        number(ui, d);
                        *d = d.abs().max(1e-12);
                    });
                }
                TickSpacing::Manual(v) => {
                    // El texto se guarda en la memoria de egui mientras se edita.
                    let key = ui.make_persistent_id(("marcas", id));
                    let mut text = ui.data_mut(|m| m.get_temp::<String>(key)).unwrap_or_else(|| {
                        v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("; ")
                    });
                    let r = ui.add(egui::TextEdit::singleline(&mut text).hint_text("1; 2,5; 4").desired_width(200.0));
                    match parse_tick_list(&text) {
                        Some(list) => *v = list,
                        None => {
                            ui.colored_label(ERROR_COLOR, "Escribe números separados por «;»");
                        }
                    }
                    if r.lost_focus() {
                        ui.data_mut(|m| m.remove::<String>(key));
                    } else {
                        ui.data_mut(|m| m.insert_temp(key, text));
                    }
                }
            }
            ui.horizontal(|ui| {
                ui.label("Menores entre principales");
                ui.add(egui::DragValue::new(&mut a.ticks.minor_per_major).range(0..=9));
            });
            combo(ui, (id, "dir"), "Dirección de las marcas", &mut a.ticks.direction, &TICK_DIRECTIONS);
        });
        ui.end_row();

        let f = &mut a.number_format;
        ui.label("Números");
        ui.vertical(|ui| {
            combo(ui, (id, "notacion"), "Notación", &mut f.notation, &NOTATIONS);
            ui.horizontal(|ui| {
                ui.label("Decimales");
                ui.add(egui::DragValue::new(&mut f.decimals).range(0..=10));
                ui.checkbox(&mut f.fixed_zerofill, "Completar con ceros");
            });
            ui.checkbox(&mut f.decimal_comma, "Coma decimal (0,95)");
            ui.checkbox(&mut f.thousands_separator, "Separar miles (1 000)");
            ui.checkbox(&mut f.common_factor, "Factor común ×10ⁿ en el extremo del eje");
        });
        ui.end_row();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_lists() {
        assert_eq!(parse_tick_list("1; 2,5; 4"), Some(vec![1.0, 2.5, 4.0]));
        assert_eq!(parse_tick_list("1 2.5 4"), Some(vec![1.0, 2.5, 4.0]));
        assert_eq!(parse_tick_list("1; x"), None);
        assert_eq!(parse_tick_list(""), None);
    }
}
