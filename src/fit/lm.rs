//! Levenberg–Marquardt con escalado diagonal (Marquardt, 1963; Moré, 1978)
//! y actualización de λ de Nielsen (1999).

use nalgebra::DVector;

use super::problem::{Outcome, Problem, small_reduction, small_step};

pub(crate) fn levenberg_marquardt(prob: &Problem, q0: &[f64], max_iter: u32, tol: f64) -> Outcome {
    let m = prob.m();
    let mut q = q0.to_vec();
    prob.clamp(&mut q);
    let Some(mut r) = prob.residuals(&q) else {
        return Outcome { q, iterations: 0, converged: false };
    };
    if m == 0 {
        return Outcome { q, iterations: 0, converged: true };
    }
    let mut chi = r.norm_squared();
    let mut lambda = -1.0;
    let mut nu = 2.0;
    // Escala de cada parámetro: máximo histórico de diag(JᵀJ) (Moré).
    let mut d = vec![0.0f64; m];
    let mut stalled_before = false;

    for it in 1..=max_iter {
        let j = prob.jacobian(&q);
        if !j.iter().all(|v| v.is_finite()) {
            return Outcome { q, iterations: it, converged: false };
        }
        let jtj = j.transpose() * &j;
        let g = j.transpose() * &r;
        let max_diag = jtj.diagonal().max().max(f64::MIN_POSITIVE);
        for k in 0..m {
            d[k] = d[k].max(jtj[(k, k)]).max(max_diag * 1e-12);
        }
        if g.amax() == 0.0 {
            return Outcome { q, iterations: it, converged: true };
        }
        // λ es adimensional porque el amortiguamiento ya va escalado por D.
        if lambda < 0.0 {
            lambda = 1e-3;
        }
        // Coseno máximo entre r y las columnas de J: ≈ 0 en un mínimo.
        let r_norm = r.norm();
        let gradient_small = (0..m).all(|k| {
            let col = jtj[(k, k)].sqrt();
            col == 0.0 || r_norm == 0.0 || g[k].abs() / (col * r_norm) <= 1e-6
        });

        // Busca un λ que reduzca χ².
        loop {
            let mut a = jtj.clone();
            for k in 0..m {
                a[(k, k)] += lambda * d[k];
            }
            let step = a.cholesky().map(|c| c.solve(&g));
            if let Some(step) = step {
                let mut q_new: Vec<f64> = q.iter().zip(step.iter()).map(|(a, b)| a + b).collect();
                prob.clamp(&mut q_new);
                let delta = DVector::from_iterator(m, q_new.iter().zip(&q).map(|(a, b)| a - b));
                let pred = delta.dot(&(DVector::from_fn(m, |k, _| lambda * d[k] * delta[k]) + &g));

                if let Some(r_new) = prob.residuals(&q_new) {
                    let chi_new = r_new.norm_squared();
                    let rho = (chi - chi_new) / pred;
                    if pred > 0.0 && rho > 0.0 {
                        // χ² estancado dos iteraciones seguidas, o paso despreciable.
                        let stalled = small_reduction(chi, chi_new, tol) && pred <= tol * chi;
                        let done = (stalled && stalled_before) || small_step(&q, delta.as_slice(), tol) && lambda < 1.0;
                        stalled_before = stalled;
                        lambda *= (1.0 - (2.0 * rho - 1.0).powi(3)).max(1.0 / 3.0);
                        nu = 2.0;
                        q = q_new;
                        r = r_new;
                        chi = chi_new;
                        if done || chi == 0.0 {
                            return Outcome { q, iterations: it, converged: true };
                        }
                        break;
                    }
                }
                // Paso diminuto que no mejora: mínimo numérico si el gradiente es ~0;
                // si no, el método está atascado.
                if small_step(&q, delta.as_slice(), tol) {
                    return Outcome { q, iterations: it, converged: gradient_small };
                }
            }
            lambda *= nu;
            nu *= 2.0;
            if !lambda.is_finite() || lambda > 1e32 * max_diag {
                return Outcome { q, iterations: it, converged: false };
            }
        }
    }
    Outcome { q, iterations: max_iter, converged: false }
}
