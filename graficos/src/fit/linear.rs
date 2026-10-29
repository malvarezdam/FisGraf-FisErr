//! Mínimos cuadrados lineales (OLS, o WLS con pesos) por SVD.
//!
//! Si f es lineal en los parámetros, f(x; q) = f₀(x) + Σ qₖ·gₖ(x) con gₖ = ∂f/∂qₖ,
//! y el mínimo de χ² es la solución de G·q = b en sentido de mínimos cuadrados.
//! Las columnas se normalizan antes de la SVD para tolerar diseños mal
//! condicionados (polinomios de grado alto).

use nalgebra::DVector;

use super::problem::Problem;

pub(crate) struct LinearSolution {
    pub q: Vec<f64>,
    /// Rango numérico de la matriz de diseño.
    pub rank: usize,
}

pub(crate) fn solve(prob: &Problem) -> Option<LinearSolution> {
    let q0 = prob.initial();
    let mut g = prob.jacobian(&q0);
    // b = √w·(y − f(q₀)) + G·q₀ = √w·(y − f₀)
    let b = prob.residuals(&q0)? + &g * DVector::from_column_slice(&q0);

    let scale: Vec<f64> = g
        .column_iter()
        .map(|c| {
            let n = c.norm();
            if n > 0.0 { n } else { 1.0 }
        })
        .collect();
    for (k, s) in scale.iter().enumerate() {
        g.column_mut(k).unscale_mut(*s);
    }

    let svd = g.svd(true, true);
    let max_sv = svd.singular_values.max();
    let eps = max_sv * f64::EPSILON * (prob.n().max(prob.m()) as f64);
    let rank = svd.singular_values.iter().filter(|&&s| s > eps).count();
    let z = svd.solve(&b, eps).ok()?;

    let q = z.iter().zip(&scale).map(|(v, s)| v / s).collect::<Vec<_>>();
    q.iter().all(|v| v.is_finite()).then_some(LinearSolution { q, rank })
}
