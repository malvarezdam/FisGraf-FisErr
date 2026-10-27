//! Métodos de ajuste: OLS/WLS, Levenberg–Marquardt, Gauss–Newton,
//! Nelder–Mead, varianza efectiva y ODR.
//!
//! [`fit`] recibe un [`Model`], los datos y las opciones, y devuelve un
//! [`FitResult`] con los parámetros, χ² y el jacobiano ponderado que la Fase 4
//! usa para la matriz de covarianza.

mod effective_var;
mod gauss_newton;
mod linear;
mod lm;
mod nelder_mead;
mod odr;
mod problem;

use nalgebra::DMatrix;

pub use crate::config::FitMethod;
use crate::config::{FitConfig, FitSettings, ModelConfig};
use crate::data::SeriesData;
use crate::model::{Model, ModelError};

use problem::Problem;

/// Datos de entrada. Los errores son incertidumbres estándar (σ).
#[derive(Debug, Clone, Copy)]
pub struct FitData<'a> {
    pub x: &'a [f64],
    pub y: &'a [f64],
    pub x_err: Option<&'a [f64]>,
    pub y_err: Option<&'a [f64]>,
}

impl<'a> From<&'a SeriesData> for FitData<'a> {
    fn from(d: &'a SeriesData) -> Self {
        Self {
            x: &d.x,
            y: &d.y,
            x_err: d.x_err.as_deref(),
            y_err: d.y_err.as_deref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FitOptions {
    pub method: FitMethod,
    pub settings: FitSettings,
    /// Valores iniciales, en el orden de los parámetros del modelo.
    pub initial: Vec<f64>,
    pub fixed: Vec<bool>,
    pub lower: Vec<Option<f64>>,
    pub upper: Vec<Option<f64>>,
}

impl FitOptions {
    /// Opciones por defecto: todos los parámetros libres y sin límites.
    pub fn new(method: FitMethod, initial: Vec<f64>) -> Self {
        let m = initial.len();
        Self {
            method,
            settings: FitSettings::default(),
            initial,
            fixed: vec![false; m],
            lower: vec![None; m],
            upper: vec![None; m],
        }
    }

    pub fn from_config(cfg: &FitConfig) -> Self {
        let ps = &cfg.model.parameters;
        Self {
            method: cfg.method,
            settings: cfg.settings.clone(),
            initial: ps.iter().map(|p| p.initial).collect(),
            fixed: ps.iter().map(|p| p.fixed).collect(),
            lower: ps.iter().map(|p| p.lower).collect(),
            upper: ps.iter().map(|p| p.upper).collect(),
        }
    }
}

/// Construye el modelo de un ajuste: los parámetros son los de la configuración.
pub fn build_model(cfg: &ModelConfig) -> Result<Model, ModelError> {
    let params: Vec<String> = cfg.parameters.iter().map(|p| p.name.clone()).collect();
    Model::new(&cfg.expression, &cfg.variable, &params, &[])
}

#[derive(Debug, Clone)]
pub struct FitResult {
    /// Método realmente usado (`Auto` ya resuelto).
    pub method: FitMethod,
    /// Todos los parámetros, en el orden del modelo (fijos incluidos).
    pub params: Vec<f64>,
    /// Índices de los parámetros libres.
    pub free: Vec<usize>,
    /// χ² ponderado (suma de cuadrados de residuos si no hay pesos).
    /// En ODR incluye el término de las correcciones de x.
    pub chi2: f64,
    /// Grados de libertad: n − parámetros libres.
    pub dof: usize,
    /// `true` si se usaron pesos 1/σ².
    pub weighted: bool,
    pub iterations: u32,
    pub converged: bool,
    /// Residuos sin ponderar, yᵢ − f(xᵢ).
    pub residuals: Vec<f64>,
    /// σ usada para cada punto (σy, σ efectiva o equivalente ODR). `None` sin pesos.
    pub sigma: Option<Vec<f64>>,
    /// Correcciones δᵢ de x halladas por ODR.
    pub x_shift: Option<Vec<f64>>,
    /// Jacobiano ponderado n × libres en la solución: covarianza = (JᵀJ)⁻¹·(escala).
    pub jacobian: DMatrix<f64>,
    pub warnings: Vec<String>,
}

impl FitResult {
    /// χ² reducido, χ²/ν. `None` si ν = 0.
    pub fn reduced_chi2(&self) -> Option<f64> {
        (self.dof > 0).then(|| self.chi2 / self.dof as f64)
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FitError {
    #[error("se esperaban {expected} valores iniciales y hay {got}")]
    ParamCount { expected: usize, got: usize },

    #[error("los datos tienen longitudes distintas")]
    LengthMismatch,

    #[error("hay {n} puntos y {m} parámetros libres: se necesitan al menos tantos puntos como parámetros")]
    TooFewPoints { n: usize, m: usize },

    #[error("el error de {axis} del punto {index} es {value}: para usar pesos todos los σ deben ser > 0 (o desactiva los pesos)")]
    BadSigma { axis: &'static str, index: usize, value: f64 },

    #[error("el valor inicial de «{0}» no es un número finito")]
    NonFiniteInitial(String),

    #[error("el valor inicial de «{name}» está fuera de sus límites")]
    InitialOutOfBounds { name: String },

    #[error("el modelo no se puede evaluar con los valores iniciales (x = {x}): revisa los valores iniciales")]
    NonFiniteStart { x: f64 },

    #[error("el ajuste lineal requiere un modelo lineal en los parámetros; usa Levenberg–Marquardt")]
    NotLinear,

    #[error("la σ efectiva de algún punto es 0 o no finita: revisa los errores de x e y")]
    BadEffectiveSigma,

    #[error("el ajuste falló: el modelo dejó de poder evaluarse")]
    Failed,
}

/// Ajusta `model` a los datos.
pub fn fit(model: &Model, data: &FitData, opts: &FitOptions) -> Result<FitResult, FitError> {
    let names = model.params();
    let total = names.len();
    if opts.initial.len() != total || opts.fixed.len() != total || opts.lower.len() != total || opts.upper.len() != total {
        return Err(FitError::ParamCount { expected: total, got: opts.initial.len() });
    }
    let n = data.x.len();
    if data.y.len() != n || data.x_err.is_some_and(|e| e.len() != n) || data.y_err.is_some_and(|e| e.len() != n) {
        return Err(FitError::LengthMismatch);
    }

    let free: Vec<usize> = (0..total).filter(|&i| !opts.fixed[i]).collect();
    let m = free.len();
    if n < m || n == 0 {
        return Err(FitError::TooFewPoints { n, m });
    }
    for (i, name) in names.iter().enumerate() {
        let v = opts.initial[i];
        if !v.is_finite() {
            return Err(FitError::NonFiniteInitial(name.clone()));
        }
        if opts.lower[i].is_some_and(|l| v < l) || opts.upper[i].is_some_and(|u| v > u) {
            return Err(FitError::InitialOutOfBounds { name: name.clone() });
        }
    }
    let p0 = &opts.initial;
    if let Some(&x) = data.x.iter().find(|&&x| !model.eval(x, p0).is_finite()) {
        return Err(FitError::NonFiniteStart { x });
    }

    let use_weights = opts.settings.use_weights;
    let check_sigmas = |errs: &[f64], axis: &'static str, allow_zero: bool| {
        errs.iter().enumerate().try_for_each(|(index, &value)| {
            let ok = value.is_finite() && (value > 0.0 || (allow_zero && value == 0.0));
            if ok { Ok(()) } else { Err(FitError::BadSigma { axis, index, value }) }
        })
    };
    let sy = if use_weights { data.y_err } else { None };
    let sx = if use_weights { data.x_err } else { None };
    let mut warnings = Vec::new();

    let has_bounds = free.iter().any(|&i| opts.lower[i].is_some() || opts.upper[i].is_some());
    let method = match opts.method {
        FitMethod::Auto if sx.is_some() => FitMethod::EffectiveVariance,
        FitMethod::Auto if model.is_linear() && !has_bounds => FitMethod::Linear,
        FitMethod::Auto => FitMethod::LevenbergMarquardt,
        FitMethod::EffectiveVariance | FitMethod::Odr if sx.is_none() => {
            warnings.push("no hay errores en x: se ajusta por mínimos cuadrados ponderados en y".into());
            if model.is_linear() && !has_bounds { FitMethod::Linear } else { FitMethod::LevenbergMarquardt }
        }
        other => other,
    };
    let x_methods = matches!(method, FitMethod::EffectiveVariance | FitMethod::Odr);
    if let Some(sy) = sy {
        check_sigmas(sy, "y", x_methods)?;
    }
    if let Some(sx) = sx {
        check_sigmas(sx, "x", true)?;
    }
    if sx.is_some() && !x_methods {
        warnings.push("los errores de x no se usan con este método: elige varianza efectiva u ODR para tenerlos en cuenta".into());
    }

    let mut prob = Problem {
        model,
        x: data.x,
        y: data.y,
        sw: match sy {
            Some(s) if !x_methods => s.iter().map(|s| 1.0 / s).collect(),
            _ => vec![1.0; n],
        },
        full: p0.clone(),
        free: free.clone(),
        lower: free.iter().map(|&i| opts.lower[i].unwrap_or(f64::NEG_INFINITY)).collect(),
        upper: free.iter().map(|&i| opts.upper[i].unwrap_or(f64::INFINITY)).collect(),
    };
    let q0 = prob.initial();
    let (max_iter, tol) = (opts.settings.max_iterations.max(1), opts.settings.tolerance.max(f64::EPSILON));

    let mut x_shift = None;
    let mut odr_chi2 = None;
    let (q, iterations, converged) = match method {
        FitMethod::Linear => {
            if !model.is_linear() {
                return Err(FitError::NotLinear);
            }
            if has_bounds {
                warnings.push("el ajuste lineal ignora los límites de los parámetros".into());
            }
            let sol = linear::solve(&prob).ok_or(FitError::Failed)?;
            if sol.rank < m {
                warnings.push(format!(
                    "la matriz de diseño tiene rango {} < {m}: hay parámetros redundantes o datos insuficientes",
                    sol.rank
                ));
            }
            (sol.q, 1, true)
        }
        FitMethod::Auto | FitMethod::LevenbergMarquardt => {
            let o = lm::levenberg_marquardt(&prob, &q0, max_iter, tol);
            (o.q, o.iterations, o.converged)
        }
        FitMethod::GaussNewton => {
            let o = gauss_newton::gauss_newton(&prob, &q0, max_iter, tol);
            (o.q, o.iterations, o.converged)
        }
        FitMethod::NelderMead => {
            // Símplex para acercarse y LM para pulir (y obtener un jacobiano fiable).
            let nm = nelder_mead::nelder_mead(&prob, &q0, max_iter, tol);
            let o = lm::levenberg_marquardt(&prob, &nm.q, max_iter, tol);
            (o.q, nm.iterations + o.iterations, o.converged)
        }
        FitMethod::EffectiveVariance => {
            let sx = sx.expect("resuelto arriba");
            let o = effective_var::effective_variance(&mut prob, &q0, sx, sy, max_iter, tol)
                .ok_or(FitError::BadEffectiveSigma)?;
            (o.q, o.iterations, o.converged)
        }
        FitMethod::Odr => {
            let sx = sx.expect("resuelto arriba");
            if sy.is_none() {
                warnings.push("ODR sin errores en y: se toma σy = 1, en las mismas unidades que σx".into());
            }
            prob.sw = sy.map_or_else(|| vec![1.0; n], |s| s.iter().map(|s| 1.0 / s).collect());
            let wx: Vec<Option<f64>> = sx.iter().map(|&s| (s > 0.0).then(|| 1.0 / s)).collect();
            let o = odr::odr(&prob, &q0, &wx, max_iter, tol).ok_or(FitError::Failed)?;
            x_shift = Some(o.delta);
            odr_chi2 = Some(o.chi2);
            (o.q, o.iterations, o.converged)
        }
    };

    // Para ODR la covarianza de β usa la σ efectiva en los x corregidos.
    if method == FitMethod::Odr {
        let shift = x_shift.as_ref().expect("ODR produce δ");
        let p = prob.expand(&q);
        prob.sw = (0..n)
            .map(|i| {
                let slope = model.dfdx(data.x[i] + shift[i], &p);
                let sy = sy.map_or(1.0, |s| s[i]);
                1.0 / (sy * sy + (slope * sx.expect("ODR con σx")[i]).powi(2)).sqrt()
            })
            .collect();
    }

    let params = prob.expand(&q);
    let residuals: Vec<f64> = (0..n).map(|i| data.y[i] - model.eval(data.x[i], &params)).collect();
    if !residuals.iter().all(|r| r.is_finite()) {
        return Err(FitError::Failed);
    }
    let chi2 = odr_chi2.unwrap_or_else(|| prob.chi2(&q));
    let weighted = sy.is_some() || x_methods;
    let jacobian = match &x_shift {
        // El jacobiano de ODR se evalúa en los x corregidos.
        Some(shift) => {
            let xs: Vec<f64> = data.x.iter().zip(shift).map(|(x, d)| x + d).collect();
            Problem { x: &xs, ..prob_view(&prob) }.jacobian(&q)
        }
        None => prob.jacobian(&q),
    };
    if !converged {
        warnings.push(format!("el ajuste no convergió en {iterations} iteraciones: prueba otros valores iniciales u otro método"));
    }
    if n == m {
        warnings.push("hay tantos puntos como parámetros: no quedan grados de libertad para estimar errores".into());
    }

    Ok(FitResult {
        method,
        params,
        free,
        chi2,
        dof: n - m,
        weighted,
        iterations,
        converged,
        residuals,
        sigma: weighted.then(|| prob.sw.iter().map(|w| 1.0 / w).collect()),
        x_shift,
        jacobian,
        warnings,
    })
}

/// Copia superficial de un problema para reevaluarlo con otros x.
fn prob_view<'a>(p: &Problem<'a>) -> Problem<'a> {
    Problem {
        model: p.model,
        x: p.x,
        y: p.y,
        sw: p.sw.clone(),
        full: p.full.clone(),
        free: p.free.clone(),
        lower: p.lower.clone(),
        upper: p.upper.clone(),
    }
}
