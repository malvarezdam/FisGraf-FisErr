//! Pestaña Estilo: tamaño, fuente, cuadrícula, leyenda, cuadro de resultados
//! y motor LaTeX.

use egui::Ui;

use crate::config::{AspectRatio, FontChoice, LegendPosition};
use crate::gui::FisGrafApp;
use crate::gui::names::ENGINES;
use crate::gui::widgets::{WARN_COLOR, accessible, combo, number, rgb_edit};

/// Lista desplegable por "tipo" para enumeraciones con datos; devuelve el
/// índice elegido si cambió.
fn kind_combo(ui: &mut Ui, id: &str, name: &str, current: usize, names: &[&str]) -> Option<usize> {
    let mut k = current;
    let r = egui::ComboBox::from_id_salt(id).selected_text(names[current]).show_ui(ui, |ui| {
        for (i, n) in names.iter().enumerate() {
            ui.selectable_value(&mut k, i, *n);
        }
    });
    accessible(&r.response, egui::Role::ComboBox, name);
    (k != current).then_some(k)
}

const ASPECTS: [&str; 7] = ["4:3", "3:2", "16:9", "9:16 (vertical)", "1:1", "Áurea (1,618:1)", "Personalizada"];

fn aspect_index(a: AspectRatio) -> usize {
    match a {
        AspectRatio::R4x3 => 0,
        AspectRatio::R3x2 => 1,
        AspectRatio::R16x9 => 2,
        AspectRatio::R9x16 => 3,
        AspectRatio::R1x1 => 4,
        AspectRatio::Golden => 5,
        AspectRatio::Custom { .. } => 6,
    }
}

const FONTS: [&str; 7] = [
    "Latin Modern (Computer Modern)",
    "Times",
    "Palatino",
    "Libertine",
    "Charter",
    "Fira Sans",
    "Fuente del sistema…",
];

fn font_index(f: &FontChoice) -> usize {
    match f {
        FontChoice::LatinModern => 0,
        FontChoice::Times => 1,
        FontChoice::Palatino => 2,
        FontChoice::Libertine => 3,
        FontChoice::Charter => 4,
        FontChoice::FiraSans => 5,
        FontChoice::System(_) => 6,
    }
}

const POSITIONS: [&str; 7] = [
    "Arriba a la derecha",
    "Arriba a la izquierda",
    "Abajo a la derecha",
    "Abajo a la izquierda",
    "Fuera, a la derecha",
    "Debajo del gráfico",
    "Personalizada",
];

fn position_index(p: LegendPosition) -> usize {
    match p {
        LegendPosition::NorthEast => 0,
        LegendPosition::NorthWest => 1,
        LegendPosition::SouthEast => 2,
        LegendPosition::SouthWest => 3,
        LegendPosition::OuterEast => 4,
        LegendPosition::Below => 5,
        LegendPosition::Custom { .. } => 6,
    }
}

fn position_edit(ui: &mut Ui, id: &str, p: &mut LegendPosition) {
    if let Some(k) = kind_combo(ui, id, "Posición", position_index(*p), &POSITIONS) {
        *p = match k {
            0 => LegendPosition::NorthEast,
            1 => LegendPosition::NorthWest,
            2 => LegendPosition::SouthEast,
            3 => LegendPosition::SouthWest,
            4 => LegendPosition::OuterEast,
            5 => LegendPosition::Below,
            _ => LegendPosition::Custom { x: 0.5, y: 0.5 },
        };
    }
    if let LegendPosition::Custom { x, y } = p {
        ui.add(egui::Slider::new(x, 0.0..=1.0).text("x (0 = izquierda)"));
        ui.add(egui::Slider::new(y, 0.0..=1.0).text("y (0 = abajo)"));
    }
}

impl FisGrafApp {
    pub(crate) fn style_panel(&mut self, ui: &mut Ui) {
        let f = &mut self.project.figure;
        ui.heading("Estilo del gráfico");
        egui::Grid::new("estilo").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
            ui.label("Título (LaTeX)");
            ui.add(egui::TextEdit::singleline(&mut f.title).hint_text("vacío = sin título").desired_width(250.0));
            ui.end_row();

            ui.label("Ancho");
            ui.add(egui::DragValue::new(&mut f.width_cm).range(2.0..=40.0).speed(0.1).suffix(" cm"));
            ui.end_row();
            ui.label("Proporción");
            ui.vertical(|ui| {
                if let Some(k) = kind_combo(ui, "proporcion", "Proporción", aspect_index(f.aspect), &ASPECTS) {
                    f.aspect = match k {
                        0 => AspectRatio::R4x3,
                        1 => AspectRatio::R3x2,
                        2 => AspectRatio::R16x9,
                        3 => AspectRatio::R9x16,
                        4 => AspectRatio::R1x1,
                        5 => AspectRatio::Golden,
                        _ => AspectRatio::Custom { width: 4.0, height: 3.0 },
                    };
                }
                if let AspectRatio::Custom { width, height } = &mut f.aspect {
                    ui.horizontal(|ui| {
                        number(ui, width);
                        ui.label(":");
                        number(ui, height);
                        *width = width.abs().max(1e-3);
                        *height = height.abs().max(1e-3);
                    });
                }
                let h = f.width_cm * f.aspect.height_over_width();
                ui.weak(format!("Área del gráfico: {:.1} × {h:.1} cm", f.width_cm));
            });
            ui.end_row();

            ui.label("Fuente");
            ui.vertical(|ui| {
                if let Some(k) = kind_combo(ui, "fuente", "Fuente", font_index(&f.font), &FONTS) {
                    f.font = match k {
                        0 => FontChoice::LatinModern,
                        1 => FontChoice::Times,
                        2 => FontChoice::Palatino,
                        3 => FontChoice::Libertine,
                        4 => FontChoice::Charter,
                        5 => FontChoice::FiraSans,
                        _ => FontChoice::System("Cambria".into()),
                    };
                }
                if let FontChoice::System(name) = &mut f.font {
                    ui.add(egui::TextEdit::singleline(name).hint_text("Nombre de la fuente instalada").desired_width(200.0));
                    if f.engine != crate::config::LatexEngine::LuaLatex {
                        ui.colored_label(WARN_COLOR, "Las fuentes del sistema requieren LuaLaTeX.");
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("Tamaño");
                    ui.add(egui::DragValue::new(&mut f.font_size_pt).range(5.0..=24.0).speed(0.1).suffix(" pt"));
                });
            });
            ui.end_row();

            ui.label("Motor LaTeX");
            combo(ui, "motor", "Motor LaTeX", &mut f.engine, &ENGINES);
            ui.end_row();

            ui.label("Cuadrícula");
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut f.grid.major, "Principal");
                    ui.checkbox(&mut f.grid.minor, "Secundaria");
                });
                ui.horizontal(|ui| {
                    rgb_edit(ui, &mut f.grid.color);
                    ui.add(egui::Slider::new(&mut f.grid.opacity, 0.05..=1.0).text("opacidad"));
                });
            });
            ui.end_row();

            ui.label("Leyenda");
            ui.vertical(|ui| {
                ui.checkbox(&mut f.legend.show, "Mostrar leyenda");
                if f.legend.show {
                    position_edit(ui, "leyenda", &mut f.legend.position);
                    ui.horizontal(|ui| {
                        ui.label("Columnas");
                        ui.add(egui::DragValue::new(&mut f.legend.columns).range(1..=6));
                        ui.checkbox(&mut f.legend.frame, "Marco");
                    });
                    ui.add(egui::Slider::new(&mut f.legend.fill_opacity, 0.0..=1.0).text("opacidad del fondo"));
                }
            });
            ui.end_row();

            ui.label("Cuadro de resultados");
            ui.vertical(|ui| {
                ui.checkbox(&mut f.results_box.show, "Mostrar")
                    .on_hover_text("Parámetros ajustados con su error; se eligen en cada ajuste");
                if f.results_box.show {
                    position_edit(ui, "resultados", &mut f.results_box.position);
                }
            });
            ui.end_row();

            ui.label("Residuos");
            ui.checkbox(&mut f.residuals_panel, "Panel de residuos debajo del gráfico");
            ui.end_row();
        });

        ui.add_space(6.0);
        egui::CollapsingHeader::new("Preámbulo LaTeX adicional").show(ui, |ui| {
            ui.weak("Paquetes o macros extra, p. ej. \\usepackage{physics}");
            ui.add(egui::TextEdit::multiline(&mut f.extra_preamble).code_editor().desired_rows(4).desired_width(f32::INFINITY));
        });
    }
}
