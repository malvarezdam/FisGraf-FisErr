//! Calculadora de propagación de errores reutilizable: la usan la pestaña
//! Propagación de FisGraf y la aplicación FisErr. Edita un
//! [`PropagationConfig`] y muestra el resultado de [`propagate`].

use egui::{RichText, Ui};
use egui_plot::{Bar, BarChart, Plot, VLine};

use crate::config::{Correlation, PropagationConfig, PropagationMethod, PropagationVariable};
use crate::gui::names::{DISTRIBUTIONS, PROPAGATION_METHODS, propagation_help};
use crate::gui::text::latex_to_plain;
use crate::gui::widgets::{ERROR_COLOR, OK_COLOR, WARN_COLOR, accessible, combo, number};
use crate::model::{parse, symbol_latex};
use crate::propagation::{
    ALL_METHODS, PropagationReport, formula_latex, propagate, sigma_expression, sigma_latex,
};
use crate::uncertainty::{SigFigs, round_gum};

/// Nombres de la ecuación (excepto `pi` y `e`), en orden alfabético.
fn expression_names(expr: &str) -> Option<Vec<String>> {
    parse(expr).ok().map(|e| e.free_symbols().into_iter().collect())
}

/// Ajusta la lista de variables a los nombres de la ecuación: agrega los
/// nuevos y quita los que ya no aparecen, conservando lo ya configurado.
pub fn sync_variables(cfg: &mut PropagationConfig) -> bool {
    let Some(names) = expression_names(&cfg.expression) else { return false };
    let old = std::mem::take(&mut cfg.variables);
    cfg.variables = names
        .into_iter()
        .map(|n| old.iter().find(|v| v.name == n).cloned().unwrap_or(PropagationVariable { name: n, ..Default::default() }))
        .collect();
    let names: Vec<&str> = cfg.variables.iter().filter(|v| !v.constant).map(|v| v.name.as_str()).collect();
    cfg.correlations.retain(|c| names.contains(&c.a.as_str()) && names.contains(&c.b.as_str()));
    true
}

/// Número con `d` decimales fijos.
fn fixed(v: f64, d: usize, comma: bool) -> String {
    let s = format!("{v:.d$}");
    if comma { s.replace('.', ",") } else { s }
}

fn method_name(m: PropagationMethod) -> &'static str {
    PROPAGATION_METHODS.iter().find(|(k, _)| *k == m).map_or("?", |(_, n)| n)
}

fn short(v: f64, comma: bool) -> String {
    let s = if v != 0.0 && (v.abs() >= 1e5 || v.abs() < 1e-3) { format!("{v:.4e}") } else { format!("{v:.6}") };
    let s = if s.contains('.') && !s.contains('e') { s.trim_end_matches('0').trim_end_matches('.').to_owned() } else { s };
    if comma { s.replace('.', ",") } else { s }
}

fn result_symbol(cfg: &PropagationConfig) -> String {
    if cfg.latex.trim().is_empty() { symbol_latex(&cfg.name) } else { cfg.latex.clone() }
}

/// Resultado en LaTeX: `$g = \left(9{,}80 \pm 0{,}05\right)\,\unit{m/s^2}$`.
pub fn result_latex(cfg: &PropagationConfig, report: &PropagationReport, comma: bool) -> String {
    let r = report.get(cfg.method);
    let rounded = round_gum(r.value, r.sigma, SigFigs::Auto, None);
    let unit = cfg.unit.trim();
    let body = if unit.is_empty() || rounded.exponent != 0 {
        rounded.latex(comma)
    } else {
        format!("\\left({}\\right)", rounded.latex(comma))
    };
    let unit = if unit.is_empty() { String::new() } else { format!("\\,\\unit{{{unit}}}") };
    format!("${} = {body}{unit}$", result_symbol(cfg))
}

/// Resultado en texto: `g = 9,80 ± 0,05 m/s^2`.
pub fn result_text(cfg: &PropagationConfig, report: &PropagationReport, comma: bool) -> String {
    let r = report.get(cfg.method);
    format!("{} = {} {}", cfg.name, round_gum(r.value, r.sigma, SigFigs::Auto, None).plain(comma), cfg.unit).trim_end().to_owned()
}

/// Estado de la calculadora: guarda el último cálculo para no repetirlo en
/// cada cuadro (Monte Carlo puede tardar).
#[derive(Default)]
pub struct Calculator {
    cache: Option<(PropagationConfig, Result<PropagationReport, String>)>,
}

impl Calculator {
    /// Resultado del cálculo para `cfg`, recalculado solo si cambió.
    pub fn report(&mut self, cfg: &PropagationConfig) -> &Result<PropagationReport, String> {
        if self.cache.as_ref().is_none_or(|(c, _)| c != cfg) {
            self.cache = Some((cfg.clone(), propagate(cfg).map_err(|e| e.to_string())));
        }
        &self.cache.as_ref().expect("recién calculado").1
    }

    /// Editor completo y resultado.
    pub fn ui(&mut self, ui: &mut Ui, cfg: &mut PropagationConfig, comma: bool) {
        editor(ui, cfg);
        ui.separator();
        let cfg = cfg.clone();
        match self.report(&cfg) {
            Err(e) => {
                ui.colored_label(ERROR_COLOR, e);
            }
            Ok(report) => report_ui(ui, &cfg, report, comma),
        }
    }
}

/// Datos del cálculo: ecuación, variables, correlaciones y método.
pub fn editor(ui: &mut Ui, cfg: &mut PropagationConfig) {
    let id = cfg.id;
    egui::Grid::new(("propagacion", id)).num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Magnitud");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut cfg.name).desired_width(60.0)).on_hover_text("Nombre del resultado");
            ui.label("LaTeX");
            ui.add(egui::TextEdit::singleline(&mut cfg.latex).hint_text(symbol_latex(&cfg.name)).desired_width(80.0));
            ui.label("Unidad");
            ui.add(egui::TextEdit::singleline(&mut cfg.unit).hint_text("m/s^2").desired_width(70.0));
        });
        ui.end_row();
        ui.label("Ecuación");
        ui.add(
            egui::TextEdit::singleline(&mut cfg.expression)
                .font(egui::TextStyle::Monospace)
                .hint_text("4*pi^2*L/T^2")
                .desired_width(280.0),
        );
        ui.end_row();
    });

    match expression_names(&cfg.expression) {
        Some(names) if names.iter().map(String::as_str).ne(cfg.variables.iter().map(|v| v.name.as_str())) => {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(WARN_COLOR, format!("Nombres en la ecuación: {}", names.join(", ")));
                if ui.button("Detectar variables").clicked() {
                    sync_variables(cfg);
                }
            });
        }
        None if !cfg.expression.trim().is_empty() => {
            if let Err(e) = parse(&cfg.expression) {
                ui.colored_label(ERROR_COLOR, format!("Error en la ecuación: {e}"));
            }
        }
        _ => {}
    }

    if !cfg.variables.is_empty() {
        ui.add_space(4.0);
        egui::Grid::new(("variables", id)).striped(true).spacing([8.0, 4.0]).show(ui, |ui| {
            for h in ["Nombre", "Tipo", "Valor", "σ (incert. estándar)", "Distribución", "Símbolo LaTeX", "Unidad"] {
                ui.strong(h);
            }
            ui.end_row();
            for v in &mut cfg.variables {
                ui.monospace(&v.name);
                let mut constant = v.constant;
                egui::ComboBox::from_id_salt(("tipo", id, &v.name))
                    .selected_text(if constant { "Constante" } else { "Variable medida" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut constant, false, "Variable medida");
                        ui.selectable_value(&mut constant, true, "Constante");
                    })
                    .response
                    .widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Tipo"));
                v.constant = constant;
                // Cada campo lleva nombre accesible (lectores de pantalla).
                let name = v.name.clone();
                accessible(&number(ui, &mut v.value), egui::Role::SpinButton, &format!("Valor de {name}"));
                if v.constant {
                    ui.weak("exacta");
                    ui.label("");
                } else {
                    accessible(&number(ui, &mut v.sigma), egui::Role::SpinButton, &format!("σ de {name}"));
                    v.sigma = v.sigma.abs();
                    combo(ui, ("dist", id, name.clone()), "Distribución", &mut v.distribution, &DISTRIBUTIONS);
                }
                let r = ui.add(egui::TextEdit::singleline(&mut v.latex).hint_text(symbol_latex(&name)).desired_width(70.0));
                accessible(&r, egui::Role::TextInput, &format!("Símbolo LaTeX de {name}"));
                let r = ui.add(egui::TextEdit::singleline(&mut v.unit).desired_width(55.0));
                accessible(&r, egui::Role::TextInput, &format!("Unidad de {name}"));
                ui.end_row();
            }
        });
        ui.weak("Si tienes la resolución del instrumento r: σ = r/(2√3) con distribución uniforme.");
    }

    let measured: Vec<String> = cfg.variables.iter().filter(|v| !v.constant).map(|v| v.name.clone()).collect();
    if measured.len() >= 2 {
        egui::CollapsingHeader::new(format!("Correlaciones ({})", cfg.correlations.len()))
            .id_salt(("correlaciones", id))
            .show(ui, |ui| {
                ui.weak("Solo si dos mediciones no son independientes (p. ej. comparten el mismo instrumento).");
                let mut remove = None;
                for (k, c) in cfg.correlations.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        for (side, name) in [("a", &mut c.a), ("b", &mut c.b)] {
                            egui::ComboBox::from_id_salt(("corr", id, k, side)).selected_text(name.as_str()).show_ui(ui, |ui| {
                                for m in &measured {
                                    ui.selectable_value(name, m.clone(), m);
                                }
                            });
                        }
                        ui.add(egui::Slider::new(&mut c.rho, -1.0..=1.0).text("ρ"));
                        if ui.small_button("✖").clicked() {
                            remove = Some(k);
                        }
                    });
                }
                if let Some(k) = remove {
                    cfg.correlations.remove(k);
                }
                if ui.button("+ Correlación").clicked() {
                    cfg.correlations.push(Correlation { a: measured[0].clone(), b: measured[1].clone(), rho: 0.5 });
                }
            });
    }

    ui.add_space(6.0);
    ui.strong("Método");
    combo(ui, ("metodo_prop", id), "Método de propagación", &mut cfg.method, &PROPAGATION_METHODS);
    ui.weak(propagation_help(cfg.method));
    let mut pct = cfg.confidence_level * 100.0;
    if ui.add(egui::Slider::new(&mut pct, 50.0..=99.9).suffix(" %").fixed_decimals(1).text("nivel de confianza")).changed() {
        cfg.confidence_level = pct / 100.0;
    }
    egui::CollapsingHeader::new("Opciones de Monte Carlo").id_salt(("mc", id)).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("Muestras");
            ui.add(egui::DragValue::new(&mut cfg.samples).range(1000..=5_000_000).speed(1000));
        });
        ui.horizontal(|ui| {
            let mut fixed = cfg.seed.is_some();
            if ui.checkbox(&mut fixed, "Semilla fija (reproducible)").changed() {
                cfg.seed = fixed.then_some(42);
            }
            if let Some(s) = &mut cfg.seed {
                ui.add(egui::DragValue::new(s));
            }
        });
    });
}

/// Resultado: valor con su error, función del error, comparación de métodos,
/// presupuesto de incertidumbre e histograma de Monte Carlo.
pub fn report_ui(ui: &mut Ui, cfg: &PropagationConfig, report: &PropagationReport, comma: bool) {
    let r = report.get(cfg.method);
    let rounded = round_gum(r.value, r.sigma, SigFigs::Auto, None);
    let unit = latex_to_plain(&cfg.unit);
    let sym = latex_to_plain(&format!("${}$", result_symbol(cfg)));
    ui.label(RichText::new(format!("{sym} = {} {unit}", rounded.plain(comma))).size(20.0).strong().color(OK_COLOR));
    let rel = if r.value != 0.0 { 100.0 * r.sigma / r.value.abs() } else { f64::NAN };
    ui.label(format!("Error relativo: {} %  ·  método: {}", fixed(rel, if rel < 1.0 { 3 } else { 2 }, comma), method_name(cfg.method)));
    let level = format!("{:.0} %", cfg.confidence_level * 100.0);
    match cfg.method {
        PropagationMethod::MaxBound => {
            ui.weak("La cota máxima no es una incertidumbre estándar: el valor verdadero está casi seguro dentro de ±cota.");
        }
        PropagationMethod::MonteCarlo => {
            ui.label(format!("Intervalo de cobertura {level}: [{}; {}] {unit}", short(r.interval.0, comma), short(r.interval.1, comma)));
        }
        _ => {
            ui.label(format!(
                "Incertidumbre expandida ({level}, k = {}): U = {} {unit}",
                fixed(report.coverage_factor, 2, comma),
                short(report.coverage_factor * r.sigma, comma)
            ));
        }
    }
    for w in &report.warnings {
        ui.colored_label(WARN_COLOR, format!("⚠ {w}"));
    }

    ui.horizontal_wrapped(|ui| {
        if ui.button("Copiar resultado LaTeX").clicked() {
            ui.ctx().copy_text(result_latex(cfg, report, comma));
        }
        if ui.button("Copiar fórmula de propagación").on_hover_text("σ_f con las derivadas parciales, en LaTeX").clicked() {
            ui.ctx().copy_text(formula_latex(cfg, report, comma));
        }
        if ui.button("Copiar texto").clicked() {
            ui.ctx().copy_text(result_text(cfg, report, comma));
        }
    });

    // La función final y la función de su error, en forma simbólica.
    ui.add_space(6.0);
    ui.strong("Función final con su error");
    let f_expr = parse(&cfg.expression).ok();
    let sigma = sigma_expression(cfg, report);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        if let Some(e) = &f_expr {
            ui.monospace(format!("{} = {e}", cfg.name));
        }
        ui.monospace(format!("σ_{} = {}", cfg.name, latex_to_plain(&sigma.to_string().replace("sigma_", "σ_"))));
    });
    ui.horizontal(|ui| {
        if ui.small_button("Copiar σ_f en LaTeX").clicked() {
            ui.ctx().copy_text(format!("${}$", sigma_latex(cfg, report, comma)));
        }
        ui.weak("(propagación lineal; con correlaciones incluye sus términos)");
    });

    ui.add_space(6.0);
    ui.strong("Comparación de métodos");
    egui::Grid::new(("comparacion", cfg.id)).striped(true).spacing([12.0, 3.0]).show(ui, |ui| {
        for h in ["Método", "Valor", "σ", &format!("Intervalo {level}")] {
            ui.strong(h);
        }
        ui.end_row();
        for m in ALL_METHODS {
            let r = report.get(m);
            let name = RichText::new(method_name(m));
            ui.label(if m == cfg.method { name.strong() } else { name });
            ui.monospace(short(r.value, comma));
            ui.monospace(if m == PropagationMethod::MaxBound { format!("±{} (cota)", short(r.sigma, comma)) } else { short(r.sigma, comma) });
            ui.monospace(format!("[{}; {}]", short(r.interval.0, comma), short(r.interval.1, comma)));
            ui.end_row();
        }
    });

    if !report.budget.is_empty() {
        ui.add_space(6.0);
        ui.strong("Presupuesto de incertidumbre");
        ui.weak("Cuánto aporta cada medición al error final (método lineal). Mejora primero la de mayor %.");
        egui::Grid::new(("presupuesto", cfg.id)).striped(true).spacing([12.0, 3.0]).show(ui, |ui| {
            for h in ["Variable", "Valor", "σ", "∂f/∂x", "|∂f/∂x|·σ", "Aporte"] {
                ui.strong(h);
            }
            ui.end_row();
            for b in &report.budget {
                ui.monospace(&b.name);
                ui.monospace(short(b.value, comma));
                ui.monospace(short(b.sigma, comma));
                ui.monospace(short(b.sensitivity, comma));
                ui.monospace(short(b.contribution.abs(), comma));
                ui.add(egui::ProgressBar::new((b.percent / 100.0) as f32).text(format!("{} %", fixed(b.percent, 1, comma))).desired_width(110.0));
                ui.end_row();
            }
        });
    }

    if !report.histogram.is_empty() {
        egui::CollapsingHeader::new("Distribución de Monte Carlo")
            .id_salt(("histograma", cfg.id))
            .default_open(cfg.method == PropagationMethod::MonteCarlo)
            .show(ui, |ui| {
                let mc = report.get(PropagationMethod::MonteCarlo);
                let bars: Vec<Bar> = report.histogram.iter().map(|(x, d)| Bar::new(*x, *d).width(report.histogram_width)).collect();
                ui.scope(|ui| {
                    *ui.visuals_mut() = egui::Visuals::light();
                    Plot::new(("mc_plot", cfg.id)).height(170.0).width(380.0).allow_scroll(false).show_y(false).show(ui, |pui| {
                        pui.bar_chart(BarChart::new("f", bars).color(egui::Color32::from_rgb(0x1F, 0x77, 0xB4)));
                        for (k, x) in [mc.interval.0, mc.interval.1].into_iter().enumerate() {
                            pui.vline(VLine::new(format!("límite {k}"), x).color(egui::Color32::from_rgb(0xD6, 0x27, 0x28)));
                        }
                        pui.vline(VLine::new("media", mc.value).color(egui::Color32::BLACK));
                    });
                });
                ui.weak("Barras: distribución simulada de f. Líneas rojas: intervalo de cobertura. Línea negra: media.");
            });
    }

    egui::CollapsingHeader::new("Derivadas parciales").id_salt(("derivadas", cfg.id)).show(ui, |ui| {
        for (n, d) in &report.derivatives {
            ui.monospace(format!("∂{}/∂{n} = {d}", cfg.name));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variables_follow_the_equation() {
        let mut c = PropagationConfig { expression: "4*pi^2*L/T^2".into(), ..Default::default() };
        assert!(sync_variables(&mut c));
        let names: Vec<_> = c.variables.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["L", "T"]);
        c.variables[0].sigma = 0.001;
        c.expression = "4*pi^2*(L + r)/T^2".into();
        sync_variables(&mut c);
        assert_eq!(c.variables.len(), 3);
        assert_eq!(c.variables[0].sigma, 0.001);
    }
}
