//! Propagación de incertidumbres de las magnitudes de entrada a una magnitud
//! calculada f(x₁, …, xₙ), por cinco métodos:
//!
//! - **Lineal** (GUM, ec. 13): u_f² = Σᵢⱼ cᵢ cⱼ u(xᵢ, xⱼ), con cᵢ = ∂f/∂xᵢ.
//!   Sin correlaciones: u_f = √(Σ (∂f/∂xᵢ)² uᵢ²).
//! - **Segundo orden** (GUM §5.1.2, nota): añade
//!   Σᵢⱼ [½(∂²f/∂xᵢ∂xⱼ)² + ∂f/∂xᵢ · ∂³f/∂xᵢ∂xⱼ²] uᵢ² uⱼ² y corrige el valor
//!   esperado con ½ Σ ∂²f/∂xᵢ² uᵢ².
//! - **Monte Carlo** (GUM, Suplemento 1): muestrea las entradas con su
//!   distribución (cópula gaussiana para las correlaciones) e informa media,
//!   desviación e intervalo de cobertura probabilísticamente simétrico.
//! - **Kragten** (Analyst 119, 1994; guía EURACHEM): cᵢuᵢ ≈ [f(xᵢ+uᵢ) − f(xᵢ−uᵢ)]/2.
//! - **Cota máxima**: Σ |∂f/∂xᵢ| uᵢ. Es una cota conservadora, no una
//!   incertidumbre estándar.
//!
//! Las derivadas son simbólicas (exactas), las mismas del módulo de ajuste.

use std::collections::HashMap;

use nalgebra::{DMatrix, DVector};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand_distr::{Distribution, StandardNormal};
use statrs::distribution::{ContinuousCDF, Normal};

use crate::config::{InputDistribution, PropagationConfig, PropagationMethod};
use crate::model::diff::diff;
use crate::model::{Expr, LatexOptions, Model, ModelError, symbol_latex, to_latex};
use crate::uncertainty::stats::normal_quantile;

/// Nombre interno de la "variable" del modelo (no se usa en la ecuación).
const DUMMY: &str = "__sin_variable";

#[derive(Debug, Clone, PartialEq)]
pub struct MethodResult {
    pub method: PropagationMethod,
    /// Estimación de f (valor nominal, o media en segundo orden y Monte Carlo).
    pub value: f64,
    /// Incertidumbre estándar (en la cota máxima, la cota).
    pub sigma: f64,
    /// Intervalo de cobertura al nivel pedido.
    pub interval: (f64, f64),
}

/// Fila del presupuesto de incertidumbre (método lineal).
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetRow {
    pub name: String,
    pub value: f64,
    pub sigma: f64,
    /// Coeficiente de sensibilidad ∂f/∂xᵢ.
    pub sensitivity: f64,
    /// cᵢ·uᵢ.
    pub contribution: f64,
    /// Porcentaje de u_f² que aporta (sin términos de correlación).
    pub percent: f64,
}

#[derive(Debug, Clone)]
pub struct PropagationReport {
    /// f evaluada en los valores de entrada.
    pub nominal: f64,
    /// Resultados de los cinco métodos, en el orden de [`ALL_METHODS`].
    pub methods: Vec<MethodResult>,
    /// Factor de cobertura (normal) para los métodos analíticos.
    pub coverage_factor: f64,
    pub budget: Vec<BudgetRow>,
    /// ∂f/∂xᵢ simbólicas, por variable medida.
    pub derivatives: Vec<(String, Expr)>,
    /// Histograma de Monte Carlo: (centro del intervalo, densidad).
    pub histogram: Vec<(f64, f64)>,
    pub histogram_width: f64,
    pub warnings: Vec<String>,
}

impl PropagationReport {
    pub fn get(&self, m: PropagationMethod) -> &MethodResult {
        self.methods.iter().find(|r| r.method == m).expect("se calculan todos los métodos")
    }
}

pub const ALL_METHODS: [PropagationMethod; 5] = [
    PropagationMethod::Linear,
    PropagationMethod::SecondOrder,
    PropagationMethod::MonteCarlo,
    PropagationMethod::Kragten,
    PropagationMethod::MaxBound,
];

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PropagationError {
    #[error("la ecuación está vacía")]
    Empty,
    #[error("{0}")]
    Model(String),
    #[error("la incertidumbre de «{0}» debe ser un número ≥ 0")]
    BadSigma(String),
    #[error("el valor de «{0}» no es un número finito")]
    BadValue(String),
    #[error("correlación inválida entre «{0}» y «{1}»: deben ser dos variables medidas distintas y |ρ| ≤ 1")]
    BadCorrelation(String, String),
    #[error("la matriz de correlación no es definida positiva: revisa los coeficientes ρ")]
    NotPositiveDefinite,
    #[error("la ecuación no se puede evaluar con estos valores (división por cero, raíz de negativo…)")]
    NonFinite,
}

fn model_error(e: ModelError) -> PropagationError {
    PropagationError::Model(match e {
        ModelError::UnusedParameter(n) => format!("la variable «{n}» no aparece en la ecuación"),
        ModelError::Unknown(names) => {
            format!("nombres sin definir: {}. Agrégalos como variable o constante", names.join(", "))
        }
        other => other.to_string(),
    })
}

/// Propaga las incertidumbres por los cinco métodos.
pub fn propagate(cfg: &PropagationConfig) -> Result<PropagationReport, PropagationError> {
    if cfg.expression.trim().is_empty() {
        return Err(PropagationError::Empty);
    }
    let mut warnings = Vec::new();
    for v in &cfg.variables {
        if !v.value.is_finite() {
            return Err(PropagationError::BadValue(v.name.clone()));
        }
        if !v.constant && !(v.sigma.is_finite() && v.sigma >= 0.0) {
            return Err(PropagationError::BadSigma(v.name.clone()));
        }
    }
    let measured: Vec<_> = cfg.variables.iter().filter(|v| !v.constant).collect();
    let names: Vec<String> = measured.iter().map(|v| v.name.clone()).collect();
    let consts: Vec<(String, f64)> = cfg.variables.iter().filter(|v| v.constant).map(|v| (v.name.clone(), v.value)).collect();
    let model = Model::new(&cfg.expression, DUMMY, &names, &consts).map_err(model_error)?;

    let m = measured.len();
    let x: Vec<f64> = measured.iter().map(|v| v.value).collect();
    let s: Vec<f64> = measured.iter().map(|v| v.sigma).collect();
    let f0 = model.eval(0.0, &x);
    if !f0.is_finite() {
        return Err(PropagationError::NonFinite);
    }
    if m == 0 {
        warnings.push("no hay variables medidas: el resultado no tiene incertidumbre".into());
    }

    // Matriz de correlación.
    let mut r = DMatrix::<f64>::identity(m, m);
    for c in &cfg.correlations {
        let (i, j) = (names.iter().position(|n| *n == c.a), names.iter().position(|n| *n == c.b));
        match (i, j) {
            (Some(i), Some(j)) if i != j && c.rho.abs() <= 1.0 => {
                r[(i, j)] = c.rho;
                r[(j, i)] = c.rho;
            }
            _ => return Err(PropagationError::BadCorrelation(c.a.clone(), c.b.clone())),
        }
    }
    let correlated = !cfg.correlations.is_empty();
    let cov = DMatrix::from_fn(m, m, |i, j| r[(i, j)] * s[i] * s[j]);

    let mut g = vec![0.0; m];
    model.gradient(0.0, &x, &mut g);
    if g.iter().any(|v| !v.is_finite()) {
        return Err(PropagationError::NonFinite);
    }
    let gv = DVector::from_column_slice(&g);
    let k = normal_quantile(0.5 + cfg.confidence_level / 2.0);

    // --- Lineal ---
    let var_lin = (gv.transpose() * &cov * &gv)[(0, 0)].max(0.0);
    let sigma_lin = var_lin.sqrt();

    // --- Segundo orden ---
    let env: HashMap<&str, f64> = names
        .iter()
        .map(String::as_str)
        .zip(x.iter().copied())
        .chain(consts.iter().map(|(n, v)| (n.as_str(), *v)))
        .collect();
    let lookup = |n: &str| env.get(n).copied();
    let first: Vec<Expr> = (0..m).map(|i| model.param_derivative(i).clone()).collect();
    let mut hess = DMatrix::<f64>::zeros(m, m);
    let mut extra = 0.0;
    for i in 0..m {
        for j in 0..m {
            let dij = diff(&first[i], &names[j]);
            let h = dij.eval_with(&lookup);
            let t = diff(&dij, &names[j]).eval_with(&lookup); // ∂³f/∂xᵢ∂xⱼ²
            hess[(i, j)] = h;
            extra += (0.5 * h * h + g[i] * t) * s[i] * s[i] * s[j] * s[j];
        }
    }
    let shift = 0.5 * (0..m).flat_map(|i| (0..m).map(move |j| (i, j))).map(|(i, j)| hess[(i, j)] * cov[(i, j)]).sum::<f64>();
    let mut var_2 = var_lin + extra;
    if correlated {
        warnings.push("segundo orden: los términos de orden superior suponen entradas independientes".into());
    }
    if !var_2.is_finite() || var_2 < 0.0 {
        warnings.push("segundo orden: la corrección no es válida aquí; se usa la varianza lineal".into());
        var_2 = var_lin;
    }
    let mean_2 = f0 + if shift.is_finite() { shift } else { 0.0 };
    let sigma_2 = var_2.sqrt();

    // --- Kragten (diferencias centrales de ±u) ---
    let delta: Vec<f64> = (0..m)
        .map(|i| {
            let (mut up, mut dn) = (x.clone(), x.clone());
            up[i] += s[i];
            dn[i] -= s[i];
            (model.eval(0.0, &up) - model.eval(0.0, &dn)) / 2.0
        })
        .collect();
    let dv = DVector::from_column_slice(&delta);
    let sigma_kragten = if delta.iter().all(|d| d.is_finite()) {
        (dv.transpose() * &r * &dv)[(0, 0)].max(0.0).sqrt()
    } else {
        warnings.push("Kragten: la ecuación no se puede evaluar a ±σ de algún valor".into());
        f64::NAN
    };

    // --- Cota máxima ---
    let bound: f64 = g.iter().zip(&s).map(|(c, u)| c.abs() * u).sum();

    // --- Monte Carlo ---
    let mc = monte_carlo(cfg, &model, &measured, &r, &mut warnings)?;

    let budget_total: f64 = g.iter().zip(&s).map(|(c, u)| (c * u).powi(2)).sum();
    let budget = measured
        .iter()
        .enumerate()
        .map(|(i, v)| BudgetRow {
            name: v.name.clone(),
            value: v.value,
            sigma: v.sigma,
            sensitivity: g[i],
            contribution: g[i] * s[i],
            percent: if budget_total > 0.0 { 100.0 * (g[i] * s[i]).powi(2) / budget_total } else { 0.0 },
        })
        .collect();

    let sym = |v: f64, u: f64| (v - k * u, v + k * u);
    let methods = vec![
        MethodResult { method: PropagationMethod::Linear, value: f0, sigma: sigma_lin, interval: sym(f0, sigma_lin) },
        MethodResult { method: PropagationMethod::SecondOrder, value: mean_2, sigma: sigma_2, interval: sym(mean_2, sigma_2) },
        MethodResult { method: PropagationMethod::MonteCarlo, value: mc.mean, sigma: mc.std, interval: mc.interval },
        MethodResult { method: PropagationMethod::Kragten, value: f0, sigma: sigma_kragten, interval: sym(f0, sigma_kragten) },
        MethodResult { method: PropagationMethod::MaxBound, value: f0, sigma: bound, interval: (f0 - bound, f0 + bound) },
    ];

    Ok(PropagationReport {
        nominal: f0,
        methods,
        coverage_factor: k,
        budget,
        derivatives: names.iter().cloned().zip(first).collect(),
        histogram: mc.histogram,
        histogram_width: mc.bin_width,
        warnings,
    })
}

struct McResult {
    mean: f64,
    std: f64,
    interval: (f64, f64),
    histogram: Vec<(f64, f64)>,
    bin_width: f64,
}

fn monte_carlo(
    cfg: &PropagationConfig,
    model: &Model,
    measured: &[&crate::config::PropagationVariable],
    r: &DMatrix<f64>,
    warnings: &mut Vec<String>,
) -> Result<McResult, PropagationError> {
    let m = measured.len();
    let n = cfg.samples.clamp(1000, 5_000_000) as usize;
    let chol = r.clone().cholesky().ok_or(PropagationError::NotPositiveDefinite)?;
    let l = chol.l();
    let phi = Normal::new(0.0, 1.0).expect("normal estándar");
    let mut rng = StdRng::seed_from_u64(cfg.seed.unwrap_or_else(rand::random));
    let mut eps = DVector::<f64>::zeros(m);
    let mut x = vec![0.0; m];
    let mut values = Vec::with_capacity(n);
    for _ in 0..n {
        for e in eps.iter_mut() {
            *e = StandardNormal.sample(&mut rng);
        }
        let z = &l * &eps;
        for (i, v) in measured.iter().enumerate() {
            let (mu, u) = (v.value, v.sigma);
            x[i] = match v.distribution {
                InputDistribution::Normal => mu + u * z[i],
                InputDistribution::Uniform => {
                    let p = phi.cdf(z[i]);
                    mu + 3f64.sqrt() * u * (2.0 * p - 1.0)
                }
                InputDistribution::Triangular => {
                    let p = phi.cdf(z[i]);
                    let a = 6f64.sqrt() * u;
                    if p < 0.5 { mu - a + a * (2.0 * p).sqrt() } else { mu + a - a * (2.0 * (1.0 - p)).sqrt() }
                }
            };
        }
        let f = model.eval(0.0, &x);
        if f.is_finite() {
            values.push(f);
        }
    }
    let lost = n - values.len();
    if lost > 0 {
        warnings.push(format!("Monte Carlo: {lost} de {n} muestras dieron un valor no finito y se descartaron"));
    }
    if values.len() < 2 {
        return Err(PropagationError::NonFinite);
    }
    let len = values.len() as f64;
    let mean = values.iter().sum::<f64>() / len;
    let std = (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (len - 1.0)).sqrt();
    values.sort_by(f64::total_cmp);
    let pct = |p: f64| {
        let pos = p * (values.len() - 1) as f64;
        let (a, b) = (pos.floor() as usize, pos.ceil() as usize);
        values[a] + (values[b] - values[a]) * (pos - a as f64)
    };
    let level = cfg.confidence_level;
    let interval = (pct((1.0 - level) / 2.0), pct((1.0 + level) / 2.0));

    // Histograma entre los percentiles 0,1 % y 99,9 %, normalizado como densidad.
    let (lo, hi) = (pct(0.001), pct(0.999));
    let bins = 60;
    let width = if hi > lo { (hi - lo) / bins as f64 } else { 1.0 };
    let mut counts = vec![0usize; bins];
    for v in &values {
        if *v >= lo && *v <= hi {
            counts[(((v - lo) / width) as usize).min(bins - 1)] += 1;
        }
    }
    let histogram = counts.iter().enumerate().map(|(i, c)| (lo + (i as f64 + 0.5) * width, *c as f64 / (len * width))).collect();
    Ok(McResult { mean, std, interval, histogram, bin_width: width })
}

/// Nombre del símbolo de la incertidumbre de una variable: `sigma_L`.
pub fn sigma_name(var: &str) -> String {
    format!("sigma_{var}")
}

/// Función del error (propagación lineal) como expresión simbólica:
/// σ_f = √(Σ (∂f/∂xᵢ)² σᵢ² + 2 Σ ρᵢⱼ (∂f/∂xᵢ)(∂f/∂xⱼ) σᵢ σⱼ).
pub fn sigma_expression(cfg: &PropagationConfig, report: &PropagationReport) -> Expr {
    use crate::model::simplify::{add, call, mul, num, pow};
    let var = |n: &str| Expr::Var(sigma_name(n));
    let deriv = |n: &str| report.derivatives.iter().find(|(m, _)| m == n).map(|(_, d)| d.clone());
    let mut sum = num(0.0);
    for (n, d) in &report.derivatives {
        sum = add(sum, mul(pow(d.clone(), num(2.0)), pow(var(n), num(2.0))));
    }
    for c in &cfg.correlations {
        if let (Some(da), Some(db)) = (deriv(&c.a), deriv(&c.b)) {
            sum = add(sum, mul(mul(num(2.0 * c.rho), mul(da, db)), mul(var(&c.a), var(&c.b))));
        }
    }
    call(crate::model::Func::Sqrt, sum)
}

/// Función del error en LaTeX, p. ej. `\sigma_{g} = \sqrt{…}`.
pub fn sigma_latex(cfg: &PropagationConfig, report: &PropagationReport, comma: bool) -> String {
    let mut opts = LatexOptions { decimal_comma: comma, ..Default::default() };
    for v in &cfg.variables {
        if !v.latex.trim().is_empty() {
            opts.symbols.insert(v.name.clone(), v.latex.clone());
            opts.symbols.insert(sigma_name(&v.name), format!("\\sigma_{{{}}}", v.latex));
        }
    }
    let f = if cfg.latex.trim().is_empty() { symbol_latex(&cfg.name) } else { cfg.latex.clone() };
    format!("\\sigma_{{{f}}} = {}", to_latex(&sigma_expression(cfg, report), &opts))
}

/// Fórmula de propagación lineal en LaTeX (entorno `align*`), con las
/// derivadas parciales explícitas.
pub fn formula_latex(cfg: &PropagationConfig, report: &PropagationReport, comma: bool) -> String {
    let mut opts = LatexOptions { decimal_comma: comma, ..Default::default() };
    for v in &cfg.variables {
        if !v.latex.trim().is_empty() {
            opts.symbols.insert(v.name.clone(), v.latex.clone());
        }
    }
    let sym = |n: &str| opts.symbols.get(n).cloned().unwrap_or_else(|| symbol_latex(n));
    let f = if cfg.latex.trim().is_empty() { symbol_latex(&cfg.name) } else { cfg.latex.clone() };
    let partial = |n: &str| format!("\\frac{{\\partial {f}}}{{\\partial {}}}", sym(n));

    let mut terms: Vec<String> = report
        .derivatives
        .iter()
        .map(|(n, _)| format!("\\left({}\\right)^{{2}} \\sigma_{{{}}}^{{2}}", partial(n), sym(n)))
        .collect();
    for c in &cfg.correlations {
        terms.push(format!(
            "2 \\rho_{{{a}{b}}} {} {} \\sigma_{{{a}}} \\sigma_{{{b}}}",
            partial(&c.a),
            partial(&c.b),
            a = sym(&c.a),
            b = sym(&c.b)
        ));
    }
    let model = crate::model::parse(&cfg.expression).ok();
    let mut out = String::from("\\begin{align*}\n");
    if let Some(e) = &model {
        out += &format!("{f} &= {} \\\\\n", to_latex(e, &opts));
    }
    out += &format!("\\sigma_{{{f}}} &= \\sqrt{{{}}} \\\\\n", terms.join(" + "));
    for (n, d) in &report.derivatives {
        out += &format!("{} &= {} \\\\\n", partial(n), to_latex(d, &opts));
    }
    out.truncate(out.trim_end_matches(" \\\\\n").len());
    out += "\n\\end{align*}\n";
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Correlation, PropagationVariable};

    fn var(name: &str, value: f64, sigma: f64) -> PropagationVariable {
        PropagationVariable { name: name.into(), value, sigma, ..Default::default() }
    }

    fn cfg(expr: &str, vars: Vec<PropagationVariable>) -> PropagationConfig {
        PropagationConfig { expression: expr.into(), variables: vars, samples: 400_000, ..Default::default() }
    }

    fn close(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * b.abs().max(1e-12)
    }

    #[test]
    fn product_relative_errors_add_in_quadrature() {
        let r = propagate(&cfg("x*y", vec![var("x", 2.0, 0.02), var("y", 5.0, 0.1)])).unwrap();
        let expected = 10.0 * ((0.01f64).powi(2) + (0.02f64).powi(2)).sqrt();
        let lin = r.get(PropagationMethod::Linear);
        assert!(close(lin.sigma, expected, 1e-12));
        assert!(close(r.get(PropagationMethod::Kragten).sigma, expected, 1e-12)); // exacto para funciones bilineales
        assert!(close(r.get(PropagationMethod::MonteCarlo).sigma, expected, 0.01));
        assert!(r.get(PropagationMethod::MaxBound).sigma >= lin.sigma);
        // Presupuesto: y aporta 4 veces más varianza que x.
        assert!(close(r.budget[1].percent, 80.0, 1e-9));
    }

    #[test]
    fn pendulum_gravity_with_constant() {
        let mut c = cfg(
            "4*pi^2*L/T^2",
            vec![var("L", 1.0, 0.001), var("T", 2.007, 0.005)],
        );
        c.name = "g".into();
        let r = propagate(&c).unwrap();
        let g = 4.0 * std::f64::consts::PI.powi(2) / 2.007f64.powi(2);
        let rel = ((0.001f64).powi(2) + (2.0 * 0.005 / 2.007f64).powi(2)).sqrt();
        assert!(close(r.nominal, g, 1e-12));
        assert!(close(r.get(PropagationMethod::Linear).sigma, g * rel, 1e-12));

        // Constante explícita: no aporta error.
        let mut c2 = cfg("k*L", vec![var("L", 2.0, 0.1), PropagationVariable { name: "k".into(), value: 3.0, constant: true, ..Default::default() }]);
        c2.samples = 1000;
        let r2 = propagate(&c2).unwrap();
        assert!(close(r2.get(PropagationMethod::Linear).sigma, 0.3, 1e-12));
        assert_eq!(r2.budget.len(), 1);
    }

    /// f = x² con x ~ N(0, 1): lineal da 0; segundo orden y Monte Carlo dan
    /// media 1 y σ = √2 (distribución χ² con 1 grado de libertad).
    #[test]
    fn second_order_captures_nonlinearity() {
        let r = propagate(&cfg("x^2", vec![var("x", 0.0, 1.0)])).unwrap();
        assert_eq!(r.get(PropagationMethod::Linear).sigma, 0.0);
        let so = r.get(PropagationMethod::SecondOrder);
        assert!(close(so.sigma, 2f64.sqrt(), 1e-12));
        assert!(close(so.value, 1.0, 1e-12));
        let mc = r.get(PropagationMethod::MonteCarlo);
        assert!(close(mc.sigma, 2f64.sqrt(), 0.02));
        assert!(close(mc.value, 1.0, 0.02));
        // Intervalo 95 % de χ²₁: [0,00098; 5,024].
        assert!(close(mc.interval.1, 5.0239, 0.03));
    }

    #[test]
    fn correlations() {
        let mut c = cfg("x - y", vec![var("x", 5.0, 0.2), var("y", 3.0, 0.2)]);
        c.correlations.push(Correlation { a: "x".into(), b: "y".into(), rho: 1.0 - 1e-12 });
        let r = propagate(&c).unwrap();
        assert!(r.get(PropagationMethod::Linear).sigma < 1e-5);
        assert!(r.get(PropagationMethod::Kragten).sigma < 1e-5);

        c.correlations[0].rho = -0.5;
        let r = propagate(&c).unwrap();
        let expected = (0.04f64 + 0.04 + 2.0 * 0.5 * 0.04).sqrt();
        assert!(close(r.get(PropagationMethod::Linear).sigma, expected, 1e-12));
        assert!(close(r.get(PropagationMethod::MonteCarlo).sigma, expected, 0.01));

        c.correlations[0].rho = 1.5;
        assert!(matches!(propagate(&c), Err(PropagationError::BadCorrelation(..))));
    }

    #[test]
    fn monte_carlo_distributions_keep_standard_uncertainty() {
        for d in [InputDistribution::Normal, InputDistribution::Uniform, InputDistribution::Triangular] {
            let mut v = var("x", 10.0, 0.5);
            v.distribution = d;
            let r = propagate(&cfg("x", vec![v])).unwrap();
            let mc = r.get(PropagationMethod::MonteCarlo);
            assert!(close(mc.sigma, 0.5, 0.01), "{d:?}: {}", mc.sigma);
        }
        // Uniforme: el intervalo 95 % es ±0,95·a con a = √3·σ.
        let mut v = var("x", 0.0, 1.0);
        v.distribution = InputDistribution::Uniform;
        let mc = propagate(&cfg("x", vec![v])).unwrap().get(PropagationMethod::MonteCarlo).clone();
        assert!(close(mc.interval.1, 0.95 * 3f64.sqrt(), 0.01));
    }

    #[test]
    fn errors_and_formula() {
        assert_eq!(propagate(&cfg("", vec![])).unwrap_err(), PropagationError::Empty);
        let e = propagate(&cfg("x*y", vec![var("x", 1.0, 0.1)])).unwrap_err();
        assert!(e.to_string().contains("nombres sin definir: y"), "{e}");
        let e = propagate(&cfg("x", vec![var("x", 1.0, 0.1), var("z", 1.0, 0.1)])).unwrap_err();
        assert!(e.to_string().contains("«z» no aparece"), "{e}");
        assert!(matches!(propagate(&cfg("sqrt(x)", vec![var("x", -1.0, 0.1)])), Err(PropagationError::NonFinite)));
        assert!(matches!(propagate(&cfg("x", vec![var("x", 1.0, -0.1)])), Err(PropagationError::BadSigma(_))));

        let mut c = cfg("4*pi^2*L/T^2", vec![var("L", 1.0, 0.001), var("T", 2.0, 0.005)]);
        c.name = "g".into();
        c.samples = 1000;
        let r = propagate(&c).unwrap();
        let tex = formula_latex(&c, &r, true);
        assert!(tex.contains(r"\sigma_{g} &= \sqrt{\left(\frac{\partial g}{\partial L}\right)^{2} \sigma_{L}^{2} + \left(\frac{\partial g}{\partial T}\right)^{2} \sigma_{T}^{2}}"), "{tex}");
        assert!(tex.contains(r"\frac{\partial g}{\partial T} &= -"), "{tex}");
        assert!(tex.ends_with("\\end{align*}\n"));
    }
}

#[cfg(test)]
mod sigma_tests {
    use super::*;
    use crate::config::PropagationVariable;

    #[test]
    fn symbolic_error_function() {
        let c = PropagationConfig {
            name: "f".into(),
            expression: "x*y".into(),
            variables: vec![
                PropagationVariable { name: "x".into(), value: 2.0, sigma: 0.1, ..Default::default() },
                PropagationVariable { name: "y".into(), value: 3.0, sigma: 0.2, ..Default::default() },
            ],
            samples: 1000,
            ..Default::default()
        };
        let r = propagate(&c).unwrap();
        let e = sigma_expression(&c, &r);
        assert_eq!(e.to_string(), "sqrt(y^2*sigma_x^2 + x^2*sigma_y^2)");
        // Evaluada en los valores medidos coincide con el método lineal.
        let env = |n: &str| match n {
            "x" => Some(2.0),
            "y" => Some(3.0),
            "sigma_x" => Some(0.1),
            "sigma_y" => Some(0.2),
            _ => None,
        };
        assert!((e.eval_with(&env) - r.get(PropagationMethod::Linear).sigma).abs() < 1e-12);
        assert_eq!(sigma_latex(&c, &r, false), r"\sigma_{f} = \sqrt{y^{2} \sigma_{x}^{2} + x^{2} \sigma_{y}^{2}}");
    }
}
