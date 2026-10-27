//! Incertidumbre de los parámetros (covarianza, bootstrap, Monte Carlo,
//! jackknife, perfil Δχ²) y estadísticos de bondad del ajuste.
//!
//! [`estimate`] toma el resultado de [`crate::fit::fit`] y calcula σ, intervalos
//! de confianza, covarianza y correlación con el método elegido.
//! [`confidence_band`] da la banda de confianza de la curva ajustada.

pub mod covariance;
mod profile;
mod resampling;
pub mod rounding;
pub mod stats;

use nalgebra::{DMatrix, DVector};

pub use crate::config::{BootstrapKind, CovarianceMode, UncertaintyConfig, UncertaintyMethod};
use crate::fit::{FitData, FitOptions, FitResult};
use crate::model::Model;
pub use rounding::{Rounded, SigFigs, round_gum};
pub use stats::{GoodnessOfFit, goodness_of_fit};

use profile::Profiler;
use resampling::{Resampler, percentile, sample_covariance};

/// Incertidumbre de un parámetro.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamUncertainty {
    pub value: f64,
    /// Incertidumbre estándar (simétrica).
    pub sigma: f64,
    /// Incertidumbre hacia abajo y hacia arriba (iguales a `sigma` salvo en el
    /// perfil de χ²).
    pub minus: f64,
    pub plus: f64,
    /// Intervalo de confianza al nivel pedido.
    pub ci_low: f64,
    pub ci_high: f64,
    /// Los parámetros fijos tienen σ = 0.
    pub fixed: bool,
}

impl ParamUncertainty {
    /// Valor y σ redondeados según la GUM.
    pub fn rounded(&self, sig: SigFigs, scientific: Option<bool>) -> Rounded {
        round_gum(self.value, if self.fixed { f64::NAN } else { self.sigma }, sig, scientific)
    }
}

#[derive(Debug, Clone)]
pub struct UncertaintyResult {
    pub method: UncertaintyMethod,
    /// Uno por parámetro del modelo, en su orden.
    pub params: Vec<ParamUncertainty>,
    /// Índices de los parámetros libres (filas/columnas de las matrices).
    pub free: Vec<usize>,
    pub covariance: DMatrix<f64>,
    pub correlation: DMatrix<f64>,
    pub confidence_level: f64,
    /// Factor que multiplica σ para obtener el intervalo (z o t de Student).
    pub coverage_factor: f64,
    /// Parámetros libres de cada re-muestreo (bootstrap y Monte Carlo).
    pub samples: Option<Vec<Vec<f64>>>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum UncertaintyError {
    #[error("no quedan grados de libertad (tantos puntos como parámetros): no se puede estimar la incertidumbre")]
    NoDegreesOfFreedom,

    #[error("Monte Carlo necesita errores de medición en x o en y")]
    NeedsErrors,

    #[error("el jackknife necesita al menos {needed} puntos")]
    TooFewPoints { needed: usize },

    #[error("solo {ok} de {total} re-muestreos convergieron: el ajuste es inestable con estos datos")]
    TooManyFailures { ok: usize, total: usize },

    #[error("el nivel de confianza debe estar entre 0 y 1 (sin incluirlos)")]
    BadLevel,
}

/// Calcula la incertidumbre de los parámetros de `result`.
///
/// `opts` deben ser las mismas opciones del ajuste original.
pub fn estimate(
    model: &Model,
    data: &FitData,
    opts: &FitOptions,
    result: &FitResult,
    cfg: &UncertaintyConfig,
) -> Result<UncertaintyResult, UncertaintyError> {
    let level = cfg.confidence_level;
    if !(level > 0.0 && level < 1.0) {
        return Err(UncertaintyError::BadLevel);
    }
    let m = result.free.len();
    let mut warnings = Vec::new();

    // Covarianza lineal: base de todos los métodos (correlación del perfil, pasos iniciales).
    let (c0, rank) = covariance::unscaled_covariance(&result.jacobian);
    if rank < m {
        warnings.push(format!(
            "el jacobiano tiene rango {rank} < {m}: algún parámetro no está determinado por los datos"
        ));
    }
    let absolute = cfg.covariance_mode == CovarianceMode::Absolute && result.weighted;
    if cfg.covariance_mode == CovarianceMode::Absolute && !result.weighted {
        warnings.push("sin errores de medición no hay σ absolutas: se escala la covarianza por χ²_red".into());
    }
    if !absolute && result.dof == 0 {
        return Err(UncertaintyError::NoDegreesOfFreedom);
    }
    let scale = if absolute { 1.0 } else { result.chi2 / result.dof as f64 };
    let linear_cov = c0 * scale;
    let k_factor = stats::coverage_factor(level, (!absolute).then_some(result.dof));

    // Opciones para re-ajustar: método resuelto y arranque en el óptimo.
    let mut refit_opts = opts.clone();
    refit_opts.method = result.method;
    refit_opts.initial = result.params.clone();
    let seed = cfg.seed.unwrap_or_else(rand::random);
    let samples_wanted = cfg.samples.max(10) as usize;

    let mut free_unc: Vec<(f64, f64, f64, f64, f64)> = Vec::with_capacity(m); // σ, −, +, ci_low, ci_high
    let mut samples = None;
    let cov = match cfg.method {
        UncertaintyMethod::Covariance => {
            for (k, &i) in result.free.iter().enumerate() {
                let s = linear_cov[(k, k)].sqrt();
                let v = result.params[i];
                free_unc.push((s, s, s, v - k_factor * s, v + k_factor * s));
            }
            linear_cov
        }
        UncertaintyMethod::Bootstrap | UncertaintyMethod::MonteCarlo => {
            let resampler = Resampler { model, data: *data, opts: refit_opts, result };
            let draws = if cfg.method == UncertaintyMethod::Bootstrap {
                resampler.bootstrap(cfg.bootstrap_kind, samples_wanted, seed)
            } else {
                if data.x_err.is_none() && data.y_err.is_none() {
                    return Err(UncertaintyError::NeedsErrors);
                }
                resampler.monte_carlo(samples_wanted, seed)
            };
            check_failures(draws.len(), samples_wanted, &mut warnings)?;
            let cov = sample_covariance(&draws, m);
            let (lo_p, hi_p) = ((1.0 - level) / 2.0, (1.0 + level) / 2.0);
            for k in 0..m {
                let mut col: Vec<f64> = draws.iter().map(|d| d[k]).collect();
                let s = cov[(k, k)].sqrt();
                free_unc.push((s, s, s, percentile(&mut col, lo_p), percentile(&mut col, hi_p)));
            }
            samples = Some(draws);
            cov
        }
        UncertaintyMethod::Jackknife => {
            let n = data.x.len();
            if n < m + 2 {
                return Err(UncertaintyError::TooFewPoints { needed: m + 2 });
            }
            let resampler = Resampler { model, data: *data, opts: refit_opts, result };
            let draws = resampler.jackknife();
            check_failures(draws.len(), n, &mut warnings)?;
            let nn = draws.len() as f64;
            let cov = sample_covariance(&draws, m) * ((nn - 1.0) * (nn - 1.0) / nn);
            let k_jack = stats::coverage_factor(level, Some(result.dof));
            for (k, &i) in result.free.iter().enumerate() {
                let s = cov[(k, k)].sqrt();
                let v = result.params[i];
                free_unc.push((s, s, s, v - k_jack * s, v + k_jack * s));
            }
            cov
        }
        UncertaintyMethod::ChiSquareProfile => {
            let profiler = Profiler { model, data: *data, opts: refit_opts, result };
            // Δχ² para 1σ y para el nivel pedido, en unidades de χ² del ajuste.
            let delta_sigma = scale;
            let delta_ci = k_factor * k_factor * scale;
            for (k, &i) in result.free.iter().enumerate() {
                let v = result.params[i];
                let s_lin = linear_cov[(k, k)].sqrt();
                let step = if s_lin.is_finite() && s_lin > 0.0 { s_lin } else { 0.1 * v.abs().max(1e-3) };
                let (m1, p1) = profiler.crossings(i, delta_sigma, step);
                let (mc, pc) = profiler.crossings(i, delta_ci, step * k_factor);
                if [m1, p1, mc, pc].iter().any(Option::is_none) {
                    warnings.push(format!(
                        "el perfil de «{}» no cruza el umbral por algún lado: el intervalo queda abierto",
                        model.params()[i]
                    ));
                }
                let (minus, plus) = (m1.unwrap_or(f64::INFINITY), p1.unwrap_or(f64::INFINITY));
                let sigma = 0.5 * (minus + plus);
                free_unc.push((
                    sigma,
                    minus,
                    plus,
                    v - mc.unwrap_or(f64::INFINITY),
                    v + pc.unwrap_or(f64::INFINITY),
                ));
            }
            linear_cov
        }
    };

    let mut params: Vec<ParamUncertainty> = result
        .params
        .iter()
        .map(|&v| ParamUncertainty { value: v, sigma: 0.0, minus: 0.0, plus: 0.0, ci_low: v, ci_high: v, fixed: true })
        .collect();
    for (k, &i) in result.free.iter().enumerate() {
        let (sigma, minus, plus, ci_low, ci_high) = free_unc[k];
        params[i] = ParamUncertainty { value: result.params[i], sigma, minus, plus, ci_low, ci_high, fixed: false };
    }

    Ok(UncertaintyResult {
        method: cfg.method,
        params,
        free: result.free.clone(),
        correlation: covariance::correlation(&cov),
        covariance: cov,
        confidence_level: level,
        coverage_factor: if cfg.method == UncertaintyMethod::Jackknife {
            stats::coverage_factor(level, Some(result.dof))
        } else {
            k_factor
        },
        samples,
        warnings,
    })
}

fn check_failures(ok: usize, total: usize, warnings: &mut Vec<String>) -> Result<(), UncertaintyError> {
    if ok < 10.max(total / 2) {
        return Err(UncertaintyError::TooManyFailures { ok, total });
    }
    if ok < total {
        warnings.push(format!("{} de {total} re-muestreos no convergieron y se descartaron", total - ok));
    }
    Ok(())
}

/// Banda de confianza de la curva ajustada en los puntos `xs`: `(inferior, superior)`.
///
/// Con bootstrap o Monte Carlo se usan percentiles de las curvas re-muestreadas.
/// Con los demás métodos, propagación lineal: Var f(x) = ∇fᵀ·C·∇f.
pub fn confidence_band(model: &Model, fit: &FitResult, unc: &UncertaintyResult, xs: &[f64]) -> Vec<(f64, f64)> {
    let level = unc.confidence_level;
    if let Some(samples) = &unc.samples {
        let mut p = fit.params.clone();
        let mut values = vec![0.0; samples.len()];
        return xs
            .iter()
            .map(|&x| {
                for (v, s) in values.iter_mut().zip(samples) {
                    for (&i, &q) in fit.free.iter().zip(s) {
                        p[i] = q;
                    }
                    *v = model.eval(x, &p);
                }
                let mut finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
                if finite.is_empty() {
                    return (f64::NAN, f64::NAN);
                }
                (percentile(&mut finite, (1.0 - level) / 2.0), percentile(&mut finite, (1.0 + level) / 2.0))
            })
            .collect();
    }

    let mut grad = vec![0.0; fit.params.len()];
    xs.iter()
        .map(|&x| {
            model.gradient(x, &fit.params, &mut grad);
            let g = DVector::from_iterator(fit.free.len(), fit.free.iter().map(|&i| grad[i]));
            let var = (g.transpose() * &unc.covariance * &g)[(0, 0)];
            let y = model.eval(x, &fit.params);
            let half = unc.coverage_factor * var.max(0.0).sqrt();
            (y - half, y + half)
        })
        .collect()
}
