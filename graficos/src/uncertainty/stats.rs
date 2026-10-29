//! Estadísticos de bondad del ajuste y cuantiles de distribuciones.

use statrs::distribution::{ChiSquared, ContinuousCDF, Normal, StudentsT};

use crate::fit::FitResult;

#[derive(Debug, Clone, PartialEq)]
pub struct GoodnessOfFit {
    /// Número de puntos.
    pub n: usize,
    /// Parámetros libres.
    pub n_free: usize,
    /// χ² ponderado (o suma de cuadrados de residuos sin pesos).
    pub chi2: f64,
    pub dof: usize,
    /// χ²/ν.
    pub reduced_chi2: Option<f64>,
    /// P(χ²_ν ≥ χ²). Solo tiene sentido con σ absolutas, así que es `None` sin pesos.
    pub p_value: Option<f64>,
    /// Suma de cuadrados de residuos sin ponderar.
    pub ssr: f64,
    /// Desviación estándar de los residuos, √(SSR/ν).
    pub residual_std: Option<f64>,
    /// Coeficiente de determinación (sin ponderar).
    pub r_squared: f64,
    pub adj_r_squared: Option<f64>,
    /// Criterio de información de Akaike.
    pub aic: f64,
    /// AIC corregido para muestras pequeñas.
    pub aicc: Option<f64>,
    /// Criterio de información bayesiano.
    pub bic: f64,
}

pub fn goodness_of_fit(fit: &FitResult, y: &[f64]) -> GoodnessOfFit {
    let n = y.len();
    let m = fit.free.len();
    let dof = fit.dof;
    let ssr: f64 = fit.residuals.iter().map(|r| r * r).sum();
    let mean = y.iter().sum::<f64>() / n as f64;
    let sst: f64 = y.iter().map(|v| (v - mean).powi(2)).sum();
    let r_squared = if sst > 0.0 { 1.0 - ssr / sst } else { f64::NAN };

    // Con σ conocidas: −2 ln L = χ² + cte. Sin pesos, σ también se estima (+1 parámetro).
    let (k, minus_2_log_l) = if fit.weighted {
        (m as f64, fit.chi2)
    } else {
        ((m + 1) as f64, n as f64 * (ssr / n as f64).max(f64::MIN_POSITIVE).ln())
    };
    let aic = minus_2_log_l + 2.0 * k;
    let nf = n as f64;

    GoodnessOfFit {
        n,
        n_free: m,
        chi2: fit.chi2,
        dof,
        reduced_chi2: fit.reduced_chi2(),
        p_value: (fit.weighted && dof > 0)
            .then(|| ChiSquared::new(dof as f64).map(|d| d.sf(fit.chi2)).ok())
            .flatten(),
        ssr,
        residual_std: (dof > 0).then(|| (ssr / dof as f64).sqrt()),
        r_squared,
        adj_r_squared: (dof > 0).then(|| 1.0 - (1.0 - r_squared) * (nf - 1.0) / dof as f64),
        aic,
        aicc: (nf - k - 1.0 > 0.0).then(|| aic + 2.0 * k * (k + 1.0) / (nf - k - 1.0)),
        bic: minus_2_log_l + k * nf.ln(),
    }
}

/// Cuantil p de la normal estándar.
pub fn normal_quantile(p: f64) -> f64 {
    Normal::new(0.0, 1.0).expect("normal estándar").inverse_cdf(p)
}

/// Cuantil p de la t de Student con `dof` grados de libertad.
pub fn t_quantile(p: f64, dof: usize) -> f64 {
    if dof == 0 {
        return f64::NAN;
    }
    StudentsT::new(0.0, 1.0, dof as f64).map_or(f64::NAN, |t| t.inverse_cdf(p))
}

/// Cuantil p de χ² con `k` grados de libertad.
pub fn chi2_quantile(p: f64, k: usize) -> f64 {
    ChiSquared::new(k as f64).map_or(f64::NAN, |d| d.inverse_cdf(p))
}

/// Factor de cobertura bilateral para un nivel de confianza: z si `dof` es
/// `None` (σ conocidas), t de Student con `dof` grados de libertad si no.
pub fn coverage_factor(level: f64, dof: Option<usize>) -> f64 {
    let p = 0.5 + level / 2.0;
    match dof {
        None => normal_quantile(p),
        Some(d) => t_quantile(p, d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantiles() {
        assert!((normal_quantile(0.975) - 1.959964).abs() < 1e-5);
        assert!((t_quantile(0.975, 10) - 2.228139).abs() < 1e-5);
        assert!((t_quantile(0.975, 1) - 12.7062).abs() < 1e-3);
        assert!((chi2_quantile(0.6826894921, 1) - 1.0).abs() < 1e-6);
        assert!((coverage_factor(0.95, None) - 1.959964).abs() < 1e-5);
        assert!(t_quantile(0.975, 0).is_nan());
    }
}
