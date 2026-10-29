//! Controles reutilizables para editar la configuración.

use egui::{Color32, Response, RichText, Ui};

use crate::config::{DashPattern, ErrorSpec, LineStyle, Marker, Rgb};
use crate::data::{CellRange, Orientation};

pub const ERROR_COLOR: Color32 = Color32::from_rgb(0xD0, 0x40, 0x40);
pub const OK_COLOR: Color32 = Color32::from_rgb(0x30, 0x9A, 0x50);

pub fn to_color32(c: Rgb) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

pub const WARN_COLOR: Color32 = Color32::from_rgb(0xC0, 0x80, 0x00);

/// Campo numérico con paso proporcional al valor (sirve igual para 9,8 que para 1e-6).
pub fn number(ui: &mut Ui, v: &mut f64) -> Response {
    let speed = (v.abs() * 0.005).max(1e-9);
    // Se muestra la forma más corta del número (0, 9.79, 1.5e-7), sin ceros de relleno.
    ui.add(egui::DragValue::new(v).speed(speed).custom_formatter(|v, _| {
        if v != 0.0 && (v.abs() >= 1e7 || v.abs() < 1e-4) { format!("{v:e}") } else { format!("{v}") }
    }))
}

/// Valor opcional: casilla para activarlo y campo numérico. `default` es el
/// valor inicial al activarlo.
pub fn optional_number(ui: &mut Ui, label: &str, v: &mut Option<f64>, default: f64) {
    let mut on = v.is_some();
    if ui.checkbox(&mut on, label).changed() {
        *v = on.then_some(v.unwrap_or(default));
    }
    if let Some(x) = v {
        number(ui, x);
    }
}

pub fn rgb_edit(ui: &mut Ui, c: &mut Rgb) -> Response {
    let mut v = [c.0, c.1, c.2];
    let r = ui.color_edit_button_srgb(&mut v);
    *c = Rgb(v[0], v[1], v[2]);
    r
}

pub const MARKERS: [(Marker, &str); 8] = [
    (Marker::Circle, "● Círculo"),
    (Marker::Square, "■ Cuadrado"),
    (Marker::Triangle, "▲ Triángulo"),
    (Marker::Diamond, "◆ Rombo"),
    (Marker::Cross, "× Cruz"),
    (Marker::Plus, "+ Más"),
    (Marker::Star, "★ Estrella"),
    (Marker::None, "Ninguno"),
];

pub const DASHES: [(DashPattern, &str); 7] = [
    (DashPattern::Solid, "Continua"),
    (DashPattern::Dashed, "Discontinua"),
    (DashPattern::DenselyDashed, "Discontinua densa"),
    (DashPattern::LooselyDashed, "Discontinua espaciada"),
    (DashPattern::Dotted, "Punteada"),
    (DashPattern::DenselyDotted, "Punteada densa"),
    (DashPattern::DashDot, "Raya-punto"),
];

fn name_of<T: PartialEq + Copy>(table: &[(T, &'static str)], v: T) -> &'static str {
    table.iter().find(|(k, _)| *k == v).map_or("?", |(_, n)| n)
}

/// Nombre accesible (lectores de pantalla) para un control sin etiqueta propia.
pub fn accessible(response: &Response, kind: egui::Role, name: &str) {
    response.widget_info(|| egui::WidgetInfo::labeled(kind, true, name));
}

/// Lista desplegable a partir de una tabla (valor, nombre). `name` es su
/// nombre accesible.
pub fn combo<T: PartialEq + Copy>(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    name: &str,
    v: &mut T,
    table: &[(T, &'static str)],
) {
    let r = egui::ComboBox::from_id_salt(id).selected_text(name_of(table, *v)).show_ui(ui, |ui| {
        for (k, label) in table {
            ui.selectable_value(v, *k, *label);
        }
    });
    accessible(&r.response, egui::Role::ComboBox, name);
}

/// Editor de estilo de línea. Con `inherit` se ofrece heredar el color de la serie.
pub fn line_style_edit(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug + Copy, line: &mut LineStyle, inherit: Option<Rgb>) {
    ui.horizontal(|ui| {
        match (inherit, line.color) {
            (Some(series), None) => {
                let mut c = series;
                ui.add_enabled_ui(false, |ui| rgb_edit(ui, &mut c));
                if ui.small_button("Elegir color").clicked() {
                    line.color = Some(series);
                }
            }
            (Some(_), Some(mut c)) => {
                rgb_edit(ui, &mut c);
                line.color = Some(c);
                if ui.small_button("Color de la serie").clicked() {
                    line.color = None;
                }
            }
            (None, color) => {
                let mut c = color.unwrap_or(Rgb::BLACK);
                rgb_edit(ui, &mut c);
                line.color = Some(c);
            }
        }
        combo(ui, (id, "trazo"), "Trazo", &mut line.dash, &DASHES);
    });
    ui.horizontal(|ui| {
        ui.label("Grosor");
        ui.add(egui::DragValue::new(&mut line.width_pt).speed(0.05).range(0.1..=10.0).suffix(" pt"));
        ui.label("Opacidad");
        ui.add(egui::Slider::new(&mut line.opacity, 0.05..=1.0).fixed_decimals(2));
    });
}

/// Resultado de validar un rango para una variable.
pub fn validate_range(text: &str) -> Result<CellRange, String> {
    let r: CellRange = text.parse().map_err(|e: crate::data::RangeError| e.to_string())?;
    r.orientation().map_err(|e| e.to_string())?;
    Ok(r)
}

/// Campo de texto para un rango de celdas con indicador de validez.
pub fn range_edit(ui: &mut Ui, text: &mut String, hint: &str) -> Response {
    ui.horizontal(|ui| {
        let r = ui.add(egui::TextEdit::singleline(text).hint_text(hint).desired_width(170.0));
        if !text.trim().is_empty() {
            match validate_range(text) {
                Ok(range) => {
                    let kind = if range.orientation() == Ok(Orientation::Row) { "fila" } else { "columna" };
                    ui.label(RichText::new("✔").color(OK_COLOR)).on_hover_text(format!("Rango válido ({kind})"));
                }
                Err(e) => {
                    ui.label(RichText::new("✖").color(ERROR_COLOR)).on_hover_text(e);
                }
            }
        }
        r
    })
    .inner
}

#[derive(Clone, Copy, PartialEq)]
enum ErrorKind {
    None,
    Range,
    Constant,
    Percent,
    Resolution,
}

const ERROR_KINDS: [(ErrorKind, &str); 5] = [
    (ErrorKind::None, "Sin error"),
    (ErrorKind::Range, "Columna del Excel"),
    (ErrorKind::Constant, "Constante"),
    (ErrorKind::Percent, "Porcentaje"),
    (ErrorKind::Resolution, "Resolución del instrumento"),
];

/// Editor de la incertidumbre de una variable.
pub fn error_spec_edit(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug + Copy, spec: &mut ErrorSpec) {
    let mut kind = match spec {
        ErrorSpec::None => ErrorKind::None,
        ErrorSpec::Range(_) => ErrorKind::Range,
        ErrorSpec::Constant(_) => ErrorKind::Constant,
        ErrorSpec::Percent(_) => ErrorKind::Percent,
        ErrorSpec::Resolution(_) => ErrorKind::Resolution,
    };
    let before = kind;
    ui.horizontal(|ui| {
        combo(ui, (id, "tipo"), "Tipo de error", &mut kind, &ERROR_KINDS);
        if kind != before {
            *spec = match kind {
                ErrorKind::None => ErrorSpec::None,
                ErrorKind::Range => ErrorSpec::Range(String::new()),
                ErrorKind::Constant => ErrorSpec::Constant(0.01),
                ErrorKind::Percent => ErrorSpec::Percent(1.0),
                ErrorKind::Resolution => ErrorSpec::Resolution(0.01),
            };
        }
        match spec {
            ErrorSpec::None => {}
            ErrorSpec::Range(r) => {
                range_edit(ui, r, "Hoja1!C2:C20");
            }
            ErrorSpec::Constant(v) => {
                ui.add(egui::DragValue::new(v).speed(0.001).range(0.0..=f64::MAX)).on_hover_text("σ igual para todas las mediciones");
            }
            ErrorSpec::Percent(v) => {
                ui.add(egui::DragValue::new(v).speed(0.1).range(0.0..=100.0).suffix(" %"));
            }
            ErrorSpec::Resolution(v) => {
                ui.add(egui::DragValue::new(v).speed(0.001).range(0.0..=f64::MAX))
                    .on_hover_text("σ = resolución / (2√3), distribución uniforme (GUM)");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_validation() {
        assert!(validate_range("Hoja1!B2:B20").is_ok());
        assert!(validate_range("B2:C20").unwrap_err().contains("una sola columna"));
        assert!(validate_range("2B").is_err());
    }
}
