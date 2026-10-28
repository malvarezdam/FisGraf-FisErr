//! Panel de resultados: parámetros con su error, estadísticos y correlaciones
//! de cada ajuste, copiables en LaTeX o texto.

use egui::{RichText, Ui};

use crate::config::{FitConfig, UncertaintyMethod};
use crate::gui::FisGrafApp;
use crate::gui::names::{FIT_METHODS, UNCERTAINTY_METHODS};
use crate::gui::text::latex_to_plain;
use crate::gui::widgets::{ERROR_COLOR, WARN_COLOR};
use crate::model::symbol_latex;
use crate::pipeline::FitOutcome;
use crate::uncertainty::{SigFigs, UncertaintyResult};

fn name_of<T: PartialEq + Copy>(table: &[(T, &'static str)], v: T) -> &'static str {
    table.iter().find(|(k, _)| *k == v).map_or("?", |(_, n)| n)
}

/// Número corto con coma o punto decimal.
fn short(v: f64, decimals: usize, comma: bool) -> String {
    let s = if v != 0.0 && (v.abs() >= 1e5 || v.abs() < 1e-3) { format!("{v:.decimals$e}") } else { format!("{v:.decimals$}") };
    if comma { s.replace('.', ",") } else { s }
}

fn symbol(cfg: &FitConfig, i: usize) -> String {
    let p = &cfg.model.parameters[i];
    if p.latex.trim().is_empty() { symbol_latex(&p.name) } else { p.latex.clone() }
}

/// Tabla LaTeX con los parámetros (redondeo GUM) y los estadísticos.
pub fn results_latex(cfg: &FitConfig, out: &FitOutcome, u: &UncertaintyResult, comma: bool) -> String {
    let num = |v: f64, d: usize| {
        let s = format!("{v:.d$}");
        if comma { s.replace('.', "{,}") } else { s }
    };
    let mut t = String::from("\\begin{tabular}{ll}\n\\hline\nParámetro & Valor \\\\\n\\hline\n");
    for (i, p) in u.params.iter().enumerate() {
        let unit = &cfg.model.parameters[i].unit;
        let value = if p.fixed {
            format!("{} \\text{{ (fijo)}}", crate::model::number_latex(p.value, comma))
        } else {
            p.rounded(SigFigs::Auto, None).latex(comma)
        };
        let unit = if unit.trim().is_empty() { String::new() } else { format!("\\,\\unit{{{unit}}}") };
        t += &format!("${}$ & ${value}${unit} \\\\\n", symbol(cfg, i));
    }
    t += "\\hline\n";
    let g = &out.goodness;
    if let Some(c) = g.reduced_chi2 {
        t += &format!("$\\chi^2_\\nu$ & ${}$ \\\\\n", num(c, 3));
    }
    if g.r_squared.is_finite() {
        t += &format!("$R^2$ & ${}$ \\\\\n", num(g.r_squared, 5));
    }
    t += "\\hline\n\\end{tabular}\n";
    t
}

/// Resumen en texto plano.
pub fn results_text(cfg: &FitConfig, out: &FitOutcome, u: &UncertaintyResult, comma: bool) -> String {
    let mut t = String::new();
    for (i, p) in u.params.iter().enumerate() {
        let pc = &cfg.model.parameters[i];
        let v = if p.fixed { format!("{} (fijo)", short(p.value, 6, comma)) } else { p.rounded(SigFigs::Auto, None).plain(comma) };
        t += format!("{} = {v} {}", pc.name, pc.unit).trim_end();
        t.push('\n');
    }
    if let Some(c) = out.goodness.reduced_chi2 {
        t += &format!("χ²_red = {}\n", short(c, 3, comma));
    }
    t += &format!("R² = {}\n", short(out.goodness.r_squared, 5, comma));
    t
}

impl FisGrafApp {
    pub(crate) fn results_panel(&mut self, ui: &mut Ui) {
        let out = self.worker.latest.clone();
        let comma = crate::pipeline::decimal_comma(&self.project);
        // Siempre dentro de un ScrollArea que no se encoge: así el panel
        // conserva su alto aunque todavía no haya resultados.
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            let Some(out) = out else {
                ui.weak("Sin resultados todavía.");
                return;
            };
            if out.computed.fits.is_empty() {
                ui.weak("No hay ajustes activos. Crea uno en la pestaña Ajustes.");
            }
            for run in &out.computed.fits {
                let Some(cfg) = out.project.fits.iter().find(|f| f.id == run.fit_id) else { continue };
                let series = out.project.series_by_id(run.series_id).map_or("?".into(), |s| latex_to_plain(&s.label));
                let title = format!("{} — {series}", latex_to_plain(&cfg.label));
                egui::CollapsingHeader::new(RichText::new(title).strong())
                    .id_salt(("resultado", run.fit_id, run.series_id))
                    .default_open(true)
                    .show(ui, |ui| match &run.outcome {
                        Err(e) => {
                            ui.colored_label(ERROR_COLOR, e);
                        }
                        Ok(o) => fit_result(ui, cfg, o, comma),
                    });
            }
        });
    }
}

fn fit_result(ui: &mut Ui, cfg: &FitConfig, o: &FitOutcome, comma: bool) {
    let r = &o.result;
    ui.weak(format!(
        "{} · {} iteraciones{} · {}",
        name_of(&FIT_METHODS, r.method),
        r.iterations,
        if r.converged { "" } else { " · NO CONVERGIÓ" },
        if r.weighted { "con pesos 1/σ²" } else { "sin pesos" }
    ));
    for w in &r.warnings {
        ui.colored_label(WARN_COLOR, format!("⚠ {w}"));
    }
    let u = match &o.uncertainty {
        Ok(u) => u,
        Err(e) => {
            ui.colored_label(ERROR_COLOR, format!("Incertidumbre: {e}"));
            return;
        }
    };
    for w in &u.warnings {
        ui.colored_label(WARN_COLOR, format!("⚠ {w}"));
    }
    let level = format!("IC {:.0} %", u.confidence_level * 100.0);
    let profile = u.method == UncertaintyMethod::ChiSquareProfile;

    ui.horizontal(|ui| {
        egui::Grid::new(("parametros_res", cfg.id, r.params.len())).striped(true).spacing([14.0, 3.0]).show(ui, |ui| {
            ui.strong("Parámetro");
            ui.strong("Valor ± σ");
            ui.strong(if profile { "−σ / +σ" } else { "σ" });
            ui.strong(&level);
            ui.end_row();
            for (i, p) in u.params.iter().enumerate() {
                let pc = &cfg.model.parameters[i];
                ui.label(latex_to_plain(&format!("${}$", symbol(cfg, i))));
                if p.fixed {
                    ui.label(format!("{} (fijo)", short(p.value, 6, comma)));
                    ui.label("—");
                    ui.label("—");
                } else {
                    let unit = latex_to_plain(&pc.unit);
                    ui.label(RichText::new(format!("{} {unit}", p.rounded(SigFigs::Auto, None).plain(comma))).monospace());
                    if profile {
                        ui.label(format!("−{} / +{}", short(p.minus, 3, comma), short(p.plus, 3, comma)));
                    } else {
                        ui.label(short(p.sigma, 4, comma));
                    }
                    ui.label(format!("[{}; {}]", short(p.ci_low, 5, comma), short(p.ci_high, 5, comma)));
                }
                ui.end_row();
            }
        });

        ui.separator();
        let g = &o.goodness;
        egui::Grid::new(("estadisticos", cfg.id, r.params.len())).spacing([10.0, 3.0]).show(ui, |ui| {
            let mut row = |name: &str, value: String| {
                ui.label(name);
                ui.monospace(value);
                ui.end_row();
            };
            row("χ²", short(g.chi2, 4, comma));
            row("ν (grados de libertad)", g.dof.to_string());
            if let Some(c) = g.reduced_chi2 {
                row("χ²_red", short(c, 3, comma));
            }
            if let Some(p) = g.p_value {
                row("p-valor", short(p, 3, comma));
            }
            row("R²", short(g.r_squared, 5, comma));
            if let Some(s) = g.residual_std {
                row("s (residuos)", short(s, 4, comma));
            }
            row("AIC / BIC", format!("{} / {}", short(g.aic, 2, comma), short(g.bic, 2, comma)));
        });
    });
    ui.weak(format!("Incertidumbre: {}", name_of(&UNCERTAINTY_METHODS, u.method)));

    if u.free.len() > 1 {
        egui::CollapsingHeader::new("Correlación entre parámetros").id_salt(("corr", cfg.id)).show(ui, |ui| {
            egui::Grid::new(("corr_tabla", cfg.id)).spacing([12.0, 2.0]).show(ui, |ui| {
                ui.label("");
                for &i in &u.free {
                    ui.strong(&cfg.model.parameters[i].name);
                }
                ui.end_row();
                for (a, &i) in u.free.iter().enumerate() {
                    ui.strong(&cfg.model.parameters[i].name);
                    for b in 0..u.free.len() {
                        let c = u.correlation[(a, b)];
                        let text = RichText::new(short(c, 3, comma)).monospace();
                        ui.label(if a != b && c.abs() > 0.95 { text.color(WARN_COLOR) } else { text });
                    }
                    ui.end_row();
                }
            });
            ui.weak("|ρ| > 0,95 (en naranja): parámetros muy correlacionados, el modelo podría estar sobreparametrizado.");
        });
    }

    ui.horizontal(|ui| {
        if ui.button("Copiar tabla LaTeX").clicked() {
            ui.ctx().copy_text(results_latex(cfg, o, u, comma));
        }
        if ui.button("Copiar texto").clicked() {
            ui.ctx().copy_text(results_text(cfg, o, u, comma));
        }
    });
    ui.add_space(4.0);
}
