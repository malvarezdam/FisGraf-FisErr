//! Pestaña Ajustes: líneas de tendencia con modelo teórico, parámetros,
//! método de ajuste y de incertidumbre.

use egui::{RichText, Ui};

use crate::config::{FitConfig, ModelConfig, ParameterConfig, UncertaintyMethod};
use crate::fit::build_model;
use crate::gui::FisGrafApp;
use crate::gui::names::{
    BOOTSTRAP_KINDS, COVARIANCE_MODES, FIT_METHODS, UNCERTAINTY_METHODS, fit_method_help, uncertainty_help,
};
use crate::gui::text::latex_to_plain;
use crate::gui::widgets::{ERROR_COLOR, OK_COLOR, WARN_COLOR, combo, line_style_edit, number, optional_number};
use crate::model::{LatexOptions, suggest_parameters};

/// Ajusta la lista de parámetros a los nombres de la ecuación: agrega los
/// nuevos (valor inicial 1) y quita los que ya no aparecen. Conserva la
/// configuración de los que siguen.
pub fn sync_parameters(model: &mut ModelConfig) -> Result<(), String> {
    let names = suggest_parameters(&model.expression, &model.variable, &[]).map_err(|e| e.to_string())?;
    let old = std::mem::take(&mut model.parameters);
    model.parameters = names
        .into_iter()
        .map(|n| old.iter().find(|p| p.name == n).cloned().unwrap_or(ParameterConfig { name: n, ..Default::default() }))
        .collect();
    Ok(())
}

impl FisGrafApp {
    pub fn add_fit(&mut self) -> u32 {
        let id = self.project.next_id();
        let series = self.selected_series.or_else(|| self.project.series.first().map(|s| s.id));
        self.project.fits.push(FitConfig {
            id,
            series_ids: series.into_iter().collect(),
            label: "Ajuste".into(),
            ..Default::default()
        });
        self.selected_fit = Some(id);
        id
    }

    pub fn remove_fit(&mut self, id: u32) {
        let Some(pos) = self.project.fits.iter().position(|f| f.id == id) else { return };
        self.project.fits.remove(pos);
        self.selected_fit = self.project.fits.get(pos.min(self.project.fits.len().saturating_sub(1))).map(|f| f.id);
    }

    pub fn duplicate_fit(&mut self, id: u32) {
        let Some(pos) = self.project.fits.iter().position(|f| f.id == id) else { return };
        let new_id = self.project.next_id();
        let mut copy = self.project.fits[pos].clone();
        copy.id = new_id;
        copy.label = format!("{} (copia)", copy.label);
        self.project.fits.insert(pos + 1, copy);
        self.selected_fit = Some(new_id);
    }

    /// Estado del último cálculo de un ajuste, por serie.
    fn fit_status(&self, fit_id: u32) -> Vec<(String, Result<String, String>, Vec<String>)> {
        let comma = crate::pipeline::decimal_comma(&self.project);
        let Some(out) = &self.worker.latest else { return Vec::new() };
        out.computed
            .fits
            .iter()
            .filter(|r| r.fit_id == fit_id)
            .map(|r| {
                let series = self.project.series_by_id(r.series_id).map_or("?".into(), |s| latex_to_plain(&s.label));
                match &r.outcome {
                    Ok(o) => {
                        let mut s = if o.result.converged {
                            format!("convergió en {} iteraciones", o.result.iterations)
                        } else {
                            "no convergió".to_owned()
                        };
                        if let Some(c) = o.result.reduced_chi2() {
                            let c = format!("{c:.3}");
                            s += &format!(" · χ²_red = {}", if comma { c.replace('.', ",") } else { c });
                        }
                        let mut warnings = o.result.warnings.clone();
                        match &o.uncertainty {
                            Ok(u) => warnings.extend(u.warnings.iter().cloned()),
                            Err(e) => warnings.push(format!("incertidumbre: {e}")),
                        }
                        (series, if o.result.converged { Ok(s) } else { Err(s) }, warnings)
                    }
                    Err(e) => (series, Err(e.clone()), Vec::new()),
                }
            })
            .collect()
    }

    pub(crate) fn fits_panel(&mut self, ui: &mut Ui) {
        if self.selected_fit.is_none_or(|id| !self.project.fits.iter().any(|f| f.id == id)) {
            self.selected_fit = self.project.fits.first().map(|f| f.id);
        }
        ui.heading("Líneas de tendencia");
        ui.weak("Ajusta una ecuación teórica a una o varias series y calcula el error de cada parámetro.");
        let sel = self.selected_fit.filter(|id| self.project.fits.iter().any(|f| f.id == *id));
        ui.horizontal(|ui| {
            if ui.button("+ Nuevo ajuste").clicked() {
                self.add_fit();
            }
            if let Some(id) = sel {
                if ui.button("Duplicar").clicked() {
                    self.duplicate_fit(id);
                }
                if ui.button("Eliminar").clicked() {
                    self.remove_fit(id);
                }
            }
        });
        if self.project.fits.is_empty() {
            ui.label("No hay ajustes todavía.");
            return;
        }
        for f in &mut self.project.fits {
            ui.horizontal(|ui| {
                ui.checkbox(&mut f.enabled, "").on_hover_text("Activo");
                let text = format!("{}  ·  {}", latex_to_plain(&f.label), f.model.expression);
                if ui.selectable_label(sel == Some(f.id), text).clicked() {
                    self.selected_fit = Some(f.id);
                }
            });
        }
        ui.separator();

        let Some(id) = self.selected_fit else { return };
        let status = self.fit_status(id);
        let series: Vec<(u32, String, crate::config::Rgb)> =
            self.project.series.iter().map(|s| (s.id, latex_to_plain(&s.label), s.color)).collect();
        let comma = crate::pipeline::decimal_comma(&self.project);
        let Some(f) = self.project.fits.iter_mut().find(|f| f.id == id) else { return };
        fit_editor(ui, f, &series, comma);

        ui.separator();
        ui.strong("Resultado");
        if status.is_empty() {
            ui.weak("Sin calcular (¿tiene series asignadas y activas?).");
        }
        for (name, st, warnings) in status {
            match st {
                Ok(s) => ui.colored_label(OK_COLOR, format!("✔ {name}: {s}")),
                Err(e) => ui.colored_label(ERROR_COLOR, format!("✖ {name}: {e}")),
            };
            for w in warnings {
                ui.colored_label(WARN_COLOR, format!("⚠ {w}"));
            }
        }
    }
}

fn fit_editor(ui: &mut Ui, f: &mut FitConfig, series: &[(u32, String, crate::config::Rgb)], comma: bool) {
    let id = f.id;
    egui::Grid::new(("ajuste", id)).num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Leyenda (LaTeX)");
        ui.vertical(|ui| {
            ui.add(egui::TextEdit::singleline(&mut f.label).desired_width(240.0));
            ui.checkbox(&mut f.show_in_legend, "En la leyenda");
        });
        ui.end_row();
        ui.label("Aplicar a");
        ui.vertical(|ui| {
            if series.is_empty() {
                ui.weak("Primero crea una serie.");
            }
            for (sid, name, _) in series {
                let mut on = f.series_ids.contains(sid);
                if ui.checkbox(&mut on, name).changed() {
                    if on {
                        f.series_ids.push(*sid);
                    } else {
                        f.series_ids.retain(|s| s != sid);
                    }
                }
            }
        });
        ui.end_row();
    });

    ui.add_space(6.0);
    ui.strong("Modelo teórico");
    egui::Grid::new(("modelo", id)).num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Ecuación");
        ui.add(
            egui::TextEdit::singleline(&mut f.model.expression)
                .font(egui::TextStyle::Monospace)
                .hint_text("2*pi*sqrt(L/g)")
                .desired_width(260.0),
        )
        .on_hover_text("Operadores + - * / ^, funciones sin cos tan exp ln log10 sqrt abs…, constantes pi y e");
        ui.end_row();
        ui.label("Variable medida");
        ui.add(egui::TextEdit::singleline(&mut f.model.variable).desired_width(80.0));
        ui.end_row();
        ui.label("LaTeX propio");
        ui.add(egui::TextEdit::singleline(&mut f.model.latex_override).hint_text("vacío = automático").desired_width(260.0));
        ui.end_row();
    });

    let missing = suggest_parameters(&f.model.expression, &f.model.variable, &[]);
    let names: Vec<&str> = f.model.parameters.iter().map(|p| p.name.as_str()).collect();
    match &missing {
        Ok(found) if found.iter().map(String::as_str).ne(names.iter().copied()) => {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(WARN_COLOR, format!("Parámetros en la ecuación: {}", found.join(", ")));
                if ui.button("Actualizar parámetros").clicked() {
                    let _ = sync_parameters(&mut f.model);
                }
            });
        }
        _ => {}
    }
    match build_model(&f.model) {
        Ok(model) => {
            let mut opts = LatexOptions { decimal_comma: comma, ..Default::default() };
            for p in &f.model.parameters {
                if !p.latex.trim().is_empty() {
                    opts.symbols.insert(p.name.clone(), p.latex.clone());
                }
            }
            let latex = if f.model.latex_override.trim().is_empty() { model.to_latex(&opts) } else { f.model.latex_override.clone() };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("f = {}", latex_to_plain(&latex))).size(15.0));
                if ui.small_button("Copiar LaTeX").clicked() {
                    ui.ctx().copy_text(latex.clone());
                }
            });
            if model.is_linear() {
                ui.weak("Lineal en los parámetros: «Automático» usará mínimos cuadrados lineales.");
            }
        }
        Err(e) => {
            ui.colored_label(ERROR_COLOR, e.to_string());
        }
    }

    if !f.model.parameters.is_empty() {
        ui.add_space(4.0);
        egui::Grid::new(("parametros", id)).striped(true).spacing([8.0, 4.0]).show(ui, |ui| {
            for h in ["Parámetro", "Inicial", "Fijo", "Límites", "Símbolo LaTeX", "Unidad (siunitx)"] {
                ui.strong(h);
            }
            ui.end_row();
            for p in &mut f.model.parameters {
                ui.monospace(&p.name);
                number(ui, &mut p.initial);
                ui.checkbox(&mut p.fixed, "").on_hover_text("No se ajusta: queda en el valor inicial");
                ui.vertical(|ui| {
                    ui.horizontal(|ui| optional_number(ui, "mín", &mut p.lower, p.initial - p.initial.abs().max(1.0)));
                    ui.horizontal(|ui| optional_number(ui, "máx", &mut p.upper, p.initial + p.initial.abs().max(1.0)));
                });
                ui.add(egui::TextEdit::singleline(&mut p.latex).hint_text(crate::model::symbol_latex(&p.name)).desired_width(90.0));
                ui.add(egui::TextEdit::singleline(&mut p.unit).hint_text("m/s^2").desired_width(70.0));
                ui.end_row();
            }
        });
    }

    ui.add_space(6.0);
    ui.strong("Método de ajuste");
    combo(ui, (id, "metodo"), "Método de ajuste", &mut f.method, &FIT_METHODS);
    ui.weak(fit_method_help(f.method));
    egui::CollapsingHeader::new("Opciones avanzadas").id_salt(("avanzadas", id)).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("Iteraciones máximas");
            ui.add(egui::DragValue::new(&mut f.settings.max_iterations).range(1..=100_000));
        });
        ui.horizontal(|ui| {
            ui.label("Tolerancia");
            ui.add(egui::DragValue::new(&mut f.settings.tolerance).speed(1e-12).range(1e-16..=1e-2).custom_formatter(|v, _| format!("{v:.0e}")));
        });
        ui.checkbox(&mut f.settings.use_weights, "Usar pesos 1/σ² (si hay errores)");
    });

    ui.add_space(6.0);
    ui.strong("Incertidumbre de los parámetros");
    let u = &mut f.uncertainty;
    combo(ui, (id, "incert"), "Método de incertidumbre", &mut u.method, &UNCERTAINTY_METHODS);
    ui.weak(uncertainty_help(u.method));
    egui::Grid::new(("incertidumbre", id)).num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Covarianza");
        combo(ui, (id, "cov"), "Covarianza", &mut u.covariance_mode, &COVARIANCE_MODES);
        ui.end_row();
        if u.method == UncertaintyMethod::Bootstrap {
            ui.label("Re-muestreo");
            combo(ui, (id, "boot"), "Tipo de bootstrap", &mut u.bootstrap_kind, &BOOTSTRAP_KINDS);
            ui.end_row();
        }
        if matches!(u.method, UncertaintyMethod::Bootstrap | UncertaintyMethod::MonteCarlo) {
            ui.label("Muestras");
            ui.add(egui::DragValue::new(&mut u.samples).range(10..=100_000).speed(10));
            ui.end_row();
            ui.label("Semilla");
            ui.horizontal(|ui| {
                let mut fixed = u.seed.is_some();
                if ui.checkbox(&mut fixed, "fija (reproducible)").changed() {
                    u.seed = fixed.then_some(42);
                }
                if let Some(s) = &mut u.seed {
                    ui.add(egui::DragValue::new(s));
                }
            });
            ui.end_row();
        }
        ui.label("Nivel de confianza");
        let mut pct = u.confidence_level * 100.0;
        if ui.add(egui::Slider::new(&mut pct, 50.0..=99.9).suffix(" %").fixed_decimals(1)).changed() {
            u.confidence_level = pct / 100.0;
        }
        ui.end_row();
    });

    ui.add_space(6.0);
    ui.strong("Curva en el gráfico");
    let inherit = f.series_ids.first().and_then(|sid| series.iter().find(|s| s.0 == *sid)).map(|s| s.2);
    line_style_edit(ui, (id, "linea"), &mut f.line, Some(inherit.unwrap_or(crate::config::Rgb::BLACK)));
    ui.horizontal(|ui| {
        ui.checkbox(&mut f.show_band, "Banda de confianza");
        if f.show_band {
            ui.add(egui::Slider::new(&mut f.band_opacity, 0.05..=0.8).text("opacidad"));
        }
    });
    ui.horizontal(|ui| {
        let mut manual = f.domain.is_some();
        if ui.checkbox(&mut manual, "Dominio manual").on_hover_text("Por defecto la curva cubre el rango de la serie").changed() {
            f.domain = manual.then_some((0.0, 1.0));
        }
        if let Some((a, b)) = &mut f.domain {
            number(ui, a);
            ui.label("a");
            number(ui, b);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Puntos de la curva");
        ui.add(egui::DragValue::new(&mut f.samples).range(2..=5000));
    });
    ui.checkbox(&mut f.show_in_results_box, "Mostrar parámetros en el cuadro de resultados del gráfico");
}
