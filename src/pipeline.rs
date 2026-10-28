//! Del proyecto al gráfico: lee las series, ajusta, estima incertidumbres y
//! arma la [`Scene`] que dibuja el generador LaTeX. La interfaz usa estas
//! mismas funciones.

use std::collections::HashMap;

use crate::config::{AxisScale, LineStyle, ProjectConfig};
use crate::data::{SeriesData, Workbook, load_series};
use crate::fit::{FitData, FitOptions, FitResult, build_model, fit};
use crate::model::{LatexOptions, Model, symbol_latex};
use crate::render::scene::{Scene, SceneBand, SceneCurve, ScenePoints, SceneResiduals};
use crate::uncertainty::{
    GoodnessOfFit, SigFigs, UncertaintyResult, confidence_band, estimate, goodness_of_fit,
};

pub struct FitOutcome {
    pub model: Model,
    pub options: FitOptions,
    pub result: FitResult,
    pub uncertainty: Result<UncertaintyResult, String>,
    pub goodness: GoodnessOfFit,
}

/// Un ajuste aplicado a una serie.
pub struct FitRun {
    pub fit_id: u32,
    pub series_id: u32,
    pub outcome: Result<FitOutcome, String>,
}

/// Resultado de leer y ajustar todo el proyecto.
pub struct Computed {
    pub series: HashMap<u32, Result<SeriesData, String>>,
    pub fits: Vec<FitRun>,
    pub references: HashMap<u32, Result<Model, String>>,
}

pub fn compute(project: &ProjectConfig, book: Option<&Workbook>) -> Computed {
    let series = project
        .series
        .iter()
        .map(|s| {
            let data = match book {
                Some(b) => load_series(b, s).map_err(|e| e.to_string()),
                None => Err("no hay un archivo Excel abierto".to_owned()),
            };
            (s.id, data)
        })
        .collect::<HashMap<_, _>>();

    let mut fits = Vec::new();
    for f in project.fits.iter().filter(|f| f.enabled) {
        for &sid in &f.series_ids {
            let outcome = match series.get(&sid) {
                None => Err(format!("la serie {sid} no existe")),
                Some(Err(e)) => Err(e.clone()),
                Some(Ok(data)) => run_fit(f, data),
            };
            fits.push(FitRun { fit_id: f.id, series_id: sid, outcome });
        }
    }

    let references = project
        .references
        .iter()
        .map(|r| {
            let consts: Vec<(String, f64)> = r.constants.iter().map(|c| (c.name.clone(), c.value)).collect();
            (r.id, Model::new(&r.expression, &r.variable, &[], &consts).map_err(|e| e.to_string()))
        })
        .collect();

    Computed { series, fits, references }
}

fn run_fit(cfg: &crate::config::FitConfig, data: &SeriesData) -> Result<FitOutcome, String> {
    let model = build_model(&cfg.model).map_err(|e| e.to_string())?;
    let options = FitOptions::from_config(cfg);
    let fd = FitData::from(data);
    let result = fit(&model, &fd, &options).map_err(|e| e.to_string())?;
    let uncertainty = estimate(&model, &fd, &options, &result, &cfg.uncertainty).map_err(|e| e.to_string());
    let goodness = goodness_of_fit(&result, &data.y);
    Ok(FitOutcome { model, options, result, uncertainty, goodness })
}

/// Símbolos LaTeX elegidos por el usuario para los parámetros de un ajuste.
pub fn latex_options(cfg: &crate::config::FitConfig, decimal_comma: bool) -> LatexOptions {
    let mut opts = LatexOptions { decimal_comma, ..Default::default() };
    for p in &cfg.model.parameters {
        if !p.latex.trim().is_empty() {
            opts.symbols.insert(p.name.clone(), p.latex.clone());
        }
    }
    opts
}

/// La coma decimal del proyecto se toma del formato del eje y.
pub fn decimal_comma(project: &ProjectConfig) -> bool {
    project.y_axis.number_format.decimal_comma
}

fn sample_x(range: (f64, f64), n: u32, log: bool) -> Vec<f64> {
    let n = n.max(2) as usize;
    let (a, b) = range;
    (0..n)
        .map(|i| {
            let t = i as f64 / (n - 1) as f64;
            if log && a > 0.0 && b > 0.0 {
                (a.ln() + t * (b.ln() - a.ln())).exp()
            } else {
                a + t * (b - a)
            }
        })
        .collect()
}

/// Intervalo x del gráfico: el manual, o el de los datos con un 5 % de margen.
fn x_range(project: &ProjectConfig, computed: &Computed) -> (f64, f64) {
    let log = project.x_axis.scale == AxisScale::Log10;
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for s in project.series.iter().filter(|s| s.visible) {
        if let Some(Ok(d)) = computed.series.get(&s.id) {
            for (i, &x) in d.x.iter().enumerate() {
                let e = d.x_err.as_ref().map_or(0.0, |e| e[i]);
                if !log || x - e > 0.0 {
                    lo = lo.min(x - e);
                }
                hi = hi.max(x + e);
            }
        }
    }
    if !lo.is_finite() || !hi.is_finite() {
        (lo, hi) = (if log { 1.0 } else { 0.0 }, 10.0);
    }
    if hi <= lo {
        let d = if lo == 0.0 { 1.0 } else { lo.abs() * 0.1 };
        (lo, hi) = (lo - d, hi + d);
    }
    let (lo, hi) = if log && lo > 0.0 {
        let f = (hi / lo).powf(0.05);
        (lo / f, hi * f)
    } else {
        let pad = 0.05 * (hi - lo);
        (lo - pad, hi + pad)
    };
    (project.x_axis.min.unwrap_or(lo), project.x_axis.max.unwrap_or(hi))
}

pub fn build_scene(project: &ProjectConfig, computed: &Computed) -> Scene {
    let mut scene = Scene { x_range: x_range(project, computed), ..Default::default() };
    let legend_on = project.figure.legend.show;
    let log = project.x_axis.scale == AxisScale::Log10;
    let comma = decimal_comma(project);
    let series_cfg: HashMap<u32, &crate::config::SeriesConfig> = project.series.iter().map(|s| (s.id, s)).collect();

    for s in &project.series {
        if let Some(Err(e)) = computed.series.get(&s.id) {
            scene.warnings.push(format!("serie «{}»: {e}", s.label));
        }
    }

    // Referencias primero: quedan debajo de ajustes y datos.
    for r in project.references.iter().filter(|r| r.enabled) {
        match computed.references.get(&r.id) {
            Some(Ok(model)) => {
                let xs = sample_x(r.domain.unwrap_or(scene.x_range), r.samples, log);
                let points = xs.iter().map(|&x| (x, model.eval(x, &[]))).collect();
                let label = if r.label.trim().is_empty() {
                    let eq = if r.latex_override.trim().is_empty() {
                        model.to_latex(&LatexOptions { decimal_comma: comma, ..Default::default() })
                    } else {
                        r.latex_override.clone()
                    };
                    format!("${eq}$")
                } else {
                    r.label.clone()
                };
                let color = r.line.color.unwrap_or(crate::config::Rgb::GRAY);
                scene.curves.push(SceneCurve {
                    legend: (legend_on && r.show_in_legend).then_some(label),
                    line: LineStyle { color: Some(color), ..r.line.clone() },
                    color,
                    points,
                    band: None,
                });
            }
            Some(Err(e)) => scene.warnings.push(format!("función de referencia: {e}")),
            None => {}
        }
    }

    for run in &computed.fits {
        let Some(cfg) = project.fits.iter().find(|f| f.id == run.fit_id) else { continue };
        let Some(series) = series_cfg.get(&run.series_id) else { continue };
        let out = match &run.outcome {
            Ok(o) => o,
            Err(e) => {
                scene.warnings.push(format!("ajuste de «{}»: {e}", series.label));
                continue;
            }
        };
        for w in &out.result.warnings {
            scene.warnings.push(format!("ajuste de «{}»: {w}", series.label));
        }
        let Some(Ok(data)) = computed.series.get(&run.series_id) else { continue };
        if !series.visible {
            continue;
        }

        let domain = cfg.domain.unwrap_or_else(|| {
            let lo = data.x.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = data.x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (lo, hi)
        });
        let xs = sample_x(domain, cfg.samples, log);
        let points = xs.iter().map(|&x| (x, out.model.eval(x, &out.result.params))).collect();
        let band = match (&out.uncertainty, cfg.show_band) {
            (Ok(u), true) => {
                let b = confidence_band(&out.model, &out.result, u, &xs);
                Some(SceneBand {
                    points: xs.iter().zip(b).map(|(&x, (lo, hi))| (x, lo, hi)).collect(),
                    opacity: cfg.band_opacity,
                })
            }
            (Err(e), true) => {
                scene.warnings.push(format!("banda de «{}»: {e}", series.label));
                None
            }
            _ => None,
        };
        let mut label = if cfg.label.trim().is_empty() { "Ajuste".to_owned() } else { cfg.label.clone() };
        if cfg.series_ids.len() > 1 {
            label = format!("{label} ({})", series.label);
        }
        let color = cfg.line.color.unwrap_or(series.color);
        scene.curves.push(SceneCurve {
            legend: (legend_on && cfg.show_in_legend).then_some(label),
            line: LineStyle { color: Some(color), ..cfg.line.clone() },
            color,
            points,
            band,
        });

        if project.figure.residuals_panel {
            scene.residuals.push(SceneResiduals {
                color: series.color,
                marker: series.marker,
                marker_size_pt: series.marker_size_pt,
                error_cap_pt: series.error_cap_pt,
                x: data.x.clone(),
                residuals: out.result.residuals.clone(),
                y_err: data.y_err.clone(),
            });
        }

        if cfg.show_in_results_box {
            match &out.uncertainty {
                Ok(u) => scene.results_box.extend(results_lines(cfg, out, u, comma)),
                Err(e) => scene.warnings.push(format!("incertidumbre de «{}»: {e}", series.label)),
            }
        }
    }

    for s in project.series.iter().filter(|s| s.visible) {
        let Some(Ok(d)) = computed.series.get(&s.id) else { continue };
        scene.points.push(ScenePoints {
            legend: (legend_on && s.show_in_legend).then(|| s.label.clone()),
            color: s.color,
            marker: s.marker,
            marker_size_pt: s.marker_size_pt,
            error_cap_pt: s.error_cap_pt,
            connect: s.connect_points.clone(),
            x: d.x.clone(),
            y: d.y.clone(),
            x_err: d.x_err.clone(),
            y_err: d.y_err.clone(),
        });
    }
    scene
}

/// `$g = \left(9{,}79 \pm 0{,}02\right)$\,\unit{m/s^2}` para cada parámetro libre.
fn results_lines(cfg: &crate::config::FitConfig, out: &FitOutcome, u: &UncertaintyResult, comma: bool) -> Vec<String> {
    let mut lines = Vec::new();
    for (i, p) in u.params.iter().enumerate() {
        if p.fixed {
            continue;
        }
        let pc = &cfg.model.parameters[i];
        let symbol = if pc.latex.trim().is_empty() { symbol_latex(&pc.name) } else { pc.latex.clone() };
        let value = p.rounded(SigFigs::Auto, None);
        let body = if value.exponent == 0 && !pc.unit.trim().is_empty() {
            format!("\\left({}\\right)", value.latex(comma))
        } else {
            value.latex(comma)
        };
        let unit = if pc.unit.trim().is_empty() { String::new() } else { format!("\\,\\unit{{{}}}", pc.unit) };
        lines.push(format!("${symbol} = {body}${unit}"));
    }
    if out.result.weighted
        && let Some(chi) = out.result.reduced_chi2()
    {
        let s = format!("{chi:.2}");
        let s = if comma { s.replace('.', "{,}") } else { s };
        lines.push(format!("$\\chi^2_\\nu = {s}$"));
    }
    lines
}
