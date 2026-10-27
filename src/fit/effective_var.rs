//! Método de la varianza efectiva (Orear, 1982): con errores en x e y se
//! ajusta por WLS con σ²_ef,i = σy,i² + (∂f/∂x · σx,i)², recalculando los pesos
//! con los parámetros nuevos hasta que se estabilizan.

use super::lm::levenberg_marquardt;
use super::problem::{Outcome, Problem, small_step};

/// σ efectivas para los parámetros `q`. `None` si alguna es nula o no finita.
pub(crate) fn effective_sigmas(prob: &Problem, q: &[f64], sx: &[f64], sy: Option<&[f64]>) -> Option<Vec<f64>> {
    let p = prob.expand(q);
    (0..prob.n())
        .map(|i| {
            let slope = prob.model.dfdx(prob.x[i], &p);
            let sy = sy.map_or(0.0, |s| s[i]);
            let s = (sy * sy + (slope * sx[i]).powi(2)).sqrt();
            (s.is_finite() && s > 0.0).then_some(s)
        })
        .collect()
}

pub(crate) fn effective_variance(
    prob: &mut Problem,
    q0: &[f64],
    sx: &[f64],
    sy: Option<&[f64]>,
    max_iter: u32,
    tol: f64,
) -> Option<Outcome> {
    let mut q = q0.to_vec();
    let mut iterations = 0;
    for _ in 0..50 {
        let sig = effective_sigmas(prob, &q, sx, sy)?;
        prob.sw = sig.iter().map(|s| 1.0 / s).collect();
        let inner = levenberg_marquardt(prob, &q, max_iter, tol);
        iterations += inner.iterations;
        let step: Vec<f64> = inner.q.iter().zip(&q).map(|(a, b)| a - b).collect();
        let stable = small_step(&q, &step, tol.sqrt().max(1e-12));
        q = inner.q;
        if !inner.converged {
            return Some(Outcome { q, iterations, converged: false });
        }
        if stable {
            // Pesos finales coherentes con los parámetros finales.
            let sig = effective_sigmas(prob, &q, sx, sy)?;
            prob.sw = sig.iter().map(|s| 1.0 / s).collect();
            return Some(Outcome { q, iterations, converged: true });
        }
    }
    Some(Outcome { q, iterations, converged: false })
}
