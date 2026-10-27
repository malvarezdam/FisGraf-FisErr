//! Gauss–Newton con búsqueda lineal por retroceso (Armijo).
//! El paso se calcula por SVD de J, sin formar JᵀJ.

use nalgebra::DVector;

use super::problem::{Outcome, Problem, small_reduction, small_step};

pub(crate) fn gauss_newton(prob: &Problem, q0: &[f64], max_iter: u32, tol: f64) -> Outcome {
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

    for it in 1..=max_iter {
        let j = prob.jacobian(&q);
        let g = j.transpose() * &r;
        let svd = j.svd(true, true);
        let eps = svd.singular_values.max() * f64::EPSILON * prob.n().max(m) as f64;
        let Ok(step) = svd.solve(&r, eps) else {
            return Outcome { q, iterations: it, converged: false };
        };
        // Pendiente de χ² en la dirección del paso: −2·gᵀδ.
        let slope = 2.0 * g.dot(&step);

        let mut alpha = 1.0;
        let mut accepted = None;
        for _ in 0..40 {
            let mut q_new: Vec<f64> = q.iter().zip(step.iter()).map(|(a, b)| a + alpha * b).collect();
            prob.clamp(&mut q_new);
            if let Some(r_new) = prob.residuals(&q_new) {
                let chi_new = r_new.norm_squared();
                if chi_new <= chi - 1e-4 * alpha * slope {
                    accepted = Some((q_new, r_new, chi_new));
                    break;
                }
            }
            alpha *= 0.5;
        }
        let Some((q_new, r_new, chi_new)) = accepted else {
            // Sin descenso posible: mínimo si el paso ya era despreciable.
            let converged = small_step(&q, step.as_slice(), tol.sqrt());
            return Outcome { q, iterations: it, converged };
        };
        let delta = DVector::from_iterator(m, q_new.iter().zip(&q).map(|(a, b)| a - b));
        let done = small_reduction(chi, chi_new, tol) || small_step(&q, delta.as_slice(), tol);
        q = q_new;
        r = r_new;
        chi = chi_new;
        if done {
            return Outcome { q, iterations: it, converged: true };
        }
    }
    Outcome { q, iterations: max_iter, converged: false }
}
