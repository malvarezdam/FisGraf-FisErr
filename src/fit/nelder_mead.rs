//! Nelder–Mead (símplex) sin derivadas, con coeficientes estándar
//! (reflexión 1, expansión 2, contracción ½, encogimiento ½).
//! Útil cuando los valores iniciales están lejos; el resultado se refina luego con LM.

use super::problem::{Outcome, Problem};

pub(crate) fn nelder_mead(prob: &Problem, q0: &[f64], max_iter: u32, tol: f64) -> Outcome {
    let m = prob.m();
    let f = |q: &[f64]| {
        let mut q = q.to_vec();
        prob.clamp(&mut q);
        prob.chi2(&q)
    };
    if m == 0 {
        return Outcome { q: q0.to_vec(), iterations: 0, converged: true };
    }

    // Símplex inicial: q₀ y un vértice desplazado 5 % en cada dirección.
    let mut simplex: Vec<(Vec<f64>, f64)> = Vec::with_capacity(m + 1);
    simplex.push((q0.to_vec(), f(q0)));
    for k in 0..m {
        let mut v = q0.to_vec();
        v[k] = if v[k] != 0.0 { v[k] * 1.05 } else { 2.5e-4 };
        let fv = f(&v);
        simplex.push((v, fv));
    }

    let max_iter = max_iter.max(200 * m as u32);
    for it in 1..=max_iter {
        simplex.sort_by(|a, b| a.1.total_cmp(&b.1));
        let (best, worst) = (simplex[0].1, simplex[m].1);

        let spread = (worst - best).abs() <= tol * (best.abs() + tol);
        let size = (1..=m).all(|i| {
            simplex[i].0.iter().zip(&simplex[0].0).all(|(a, b)| (a - b).abs() <= tol.sqrt() * (b.abs() + tol))
        });
        if spread && size {
            return finish(prob, simplex, it, true);
        }

        let centroid: Vec<f64> = (0..m).map(|k| simplex[..m].iter().map(|v| v.0[k]).sum::<f64>() / m as f64).collect();
        let along = |t: f64| -> Vec<f64> {
            centroid.iter().zip(&simplex[m].0).map(|(c, w)| c + t * (c - w)).collect()
        };

        let xr = along(1.0);
        let fr = f(&xr);
        if fr < best {
            let xe = along(2.0);
            let fe = f(&xe);
            simplex[m] = if fe < fr { (xe, fe) } else { (xr, fr) };
        } else if fr < simplex[m - 1].1 {
            simplex[m] = (xr, fr);
        } else {
            let (xc, fc) = if fr < worst {
                let xc = along(0.5);
                let fc = f(&xc);
                (xc, fc)
            } else {
                let xc = along(-0.5);
                let fc = f(&xc);
                (xc, fc)
            };
            if fc < worst.min(fr) {
                simplex[m] = (xc, fc);
            } else {
                // Encoge hacia el mejor vértice.
                let b = simplex[0].0.clone();
                for v in simplex.iter_mut().skip(1) {
                    for (vk, bk) in v.0.iter_mut().zip(&b) {
                        *vk = bk + 0.5 * (*vk - bk);
                    }
                    v.1 = f(&v.0);
                }
            }
        }
    }
    simplex.sort_by(|a, b| a.1.total_cmp(&b.1));
    finish(prob, simplex, max_iter, false)
}

fn finish(prob: &Problem, mut simplex: Vec<(Vec<f64>, f64)>, iterations: u32, converged: bool) -> Outcome {
    let mut q = simplex.swap_remove(0).0;
    prob.clamp(&mut q);
    Outcome { q, iterations, converged }
}
