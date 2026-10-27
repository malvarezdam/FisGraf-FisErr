//! Regresión de distancia ortogonal ponderada (Boggs, Byrd y Schnabel, 1987;
//! el método de ODRPACK y `scipy.odr`).
//!
//! Minimiza Σ [ (f(xᵢ + δᵢ; β) − yᵢ)²/σy,i² + δᵢ²/σx,i² ] sobre β y las
//! correcciones δᵢ de x. Cada paso de Levenberg–Marquardt elimina las δ con el
//! complemento de Schur, así el costo es O(n·m²) y no O((n + m)³).

use nalgebra::{DMatrix, DVector};

use super::problem::{Problem, small_reduction, small_step};

pub(crate) struct OdrOutcome {
    pub q: Vec<f64>,
    pub delta: Vec<f64>,
    pub chi2: f64,
    pub iterations: u32,
    pub converged: bool,
}

struct State {
    q: Vec<f64>,
    delta: Vec<f64>,
    /// e1ᵢ = wyᵢ·(f(xᵢ + δᵢ) − yᵢ)
    e1: Vec<f64>,
    chi2: f64,
}

/// `prob.sw` contiene 1/σy (o 1). `wx[i]` es 1/σx,i, o `None` si x es exacta.
pub(crate) fn odr(prob: &Problem, q0: &[f64], wx: &[Option<f64>], max_iter: u32, tol: f64) -> Option<OdrOutcome> {
    let (n, m) = (prob.n(), prob.m());
    let wy = &prob.sw;

    let evaluate = |q: &[f64], delta: &[f64]| -> Option<State> {
        let p = prob.expand(q);
        let mut chi2 = 0.0;
        let mut e1 = Vec::with_capacity(n);
        for i in 0..n {
            let e = wy[i] * (prob.model.eval(prob.x[i] + delta[i], &p) - prob.y[i]);
            let e2 = wx[i].map_or(0.0, |w| w * delta[i]);
            chi2 += e * e + e2 * e2;
            e1.push(e);
        }
        chi2.is_finite().then(|| State { q: q.to_vec(), delta: delta.to_vec(), e1, chi2 })
    };

    let mut q = q0.to_vec();
    prob.clamp(&mut q);
    let mut s = evaluate(&q, &vec![0.0; n])?;
    let mut lambda = -1.0;
    let mut nu = 2.0;
    let mut d_beta = vec![0.0f64; m];
    let mut d_delta = vec![0.0f64; n];
    let mut stalled_before = false;
    let mut grad = vec![0.0; prob.full.len()];

    for it in 1..=max_iter {
        let p = prob.expand(&s.q);
        // Por punto: Jᵢ = wyᵢ·∂f/∂β, vᵢ = wyᵢ·f'ᵢ, sᵢ = wx,i.
        let mut jac = DMatrix::zeros(n, m);
        let mut v = vec![0.0; n];
        for i in 0..n {
            let xi = prob.x[i] + s.delta[i];
            prob.model.gradient(xi, &p, &mut grad);
            for (k, &idx) in prob.free.iter().enumerate() {
                jac[(i, k)] = wy[i] * grad[idx];
            }
            v[i] = wy[i] * prob.model.dfdx(xi, &p);
        }
        if !jac.iter().chain(&v).all(|x| x.is_finite()) {
            return Some(done(s, it, false));
        }

        let h_bb = jac.transpose() * &jac;
        let g_b = jac.transpose() * DVector::from_column_slice(&s.e1);
        let g_d: Vec<f64> = (0..n)
            .map(|i| v[i] * s.e1[i] + wx[i].map_or(0.0, |w| w * w * s.delta[i]))
            .collect();
        let h_dd: Vec<f64> = (0..n).map(|i| v[i] * v[i] + wx[i].map_or(0.0, |w| w * w)).collect();

        let max_diag = h_bb.diagonal().max().max(h_dd.iter().copied().fold(0.0, f64::max)).max(f64::MIN_POSITIVE);
        for k in 0..m {
            d_beta[k] = d_beta[k].max(h_bb[(k, k)]).max(max_diag * 1e-12);
        }
        for i in 0..n {
            d_delta[i] = d_delta[i].max(h_dd[i]).max(max_diag * 1e-12);
        }
        // λ adimensional: el amortiguamiento ya va escalado por D.
        if lambda < 0.0 {
            lambda = 1e-3;
        }

        loop {
            // Complemento de Schur: (H_ββ + λD − Σ vᵢ² JᵢᵀJᵢ / hᵢ)·Δβ = −g_β + Σ vᵢ Jᵢᵀ g_δᵢ / hᵢ
            let mut a = h_bb.clone();
            let mut rhs = -&g_b;
            let mut h = vec![0.0; n];
            for k in 0..m {
                a[(k, k)] += lambda * d_beta[k];
            }
            for i in 0..n {
                if wx[i].is_none() {
                    continue;
                }
                h[i] = h_dd[i] + lambda * d_delta[i];
                let row = jac.row(i);
                let c = v[i] * v[i] / h[i];
                a -= c * row.transpose() * row;
                rhs += (v[i] * g_d[i] / h[i]) * row.transpose();
            }

            if let Some(db) = a.cholesky().map(|c| c.solve(&rhs)) {
                let mut q_new: Vec<f64> = s.q.iter().zip(db.iter()).map(|(a, b)| a + b).collect();
                prob.clamp(&mut q_new);
                let db = DVector::from_iterator(m, q_new.iter().zip(&s.q).map(|(a, b)| a - b));
                let dd: Vec<f64> = (0..n)
                    .map(|i| if wx[i].is_some() { -(g_d[i] + v[i] * jac.row(i).dot(&db.transpose())) / h[i] } else { 0.0 })
                    .collect();
                let delta_new: Vec<f64> = s.delta.iter().zip(&dd).map(|(a, b)| a + b).collect();

                // Reducción predicha: Δᵀ(λDΔ − g)
                let pred = (0..m).map(|k| db[k] * (lambda * d_beta[k] * db[k] - g_b[k])).sum::<f64>()
                    + (0..n).map(|i| dd[i] * (lambda * d_delta[i] * dd[i] - g_d[i])).sum::<f64>();

                if let Some(new) = evaluate(&q_new, &delta_new) {
                    let rho = (s.chi2 - new.chi2) / pred;
                    if pred > 0.0 && rho > 0.0 {
                        let stalled = small_reduction(s.chi2, new.chi2, tol) && pred <= tol * s.chi2;
                        let finished = (stalled && stalled_before) || (small_step(&s.q, db.as_slice(), tol) && lambda < 1.0);
                        stalled_before = stalled;
                        lambda *= (1.0 - (2.0 * rho - 1.0).powi(3)).max(1.0 / 3.0);
                        nu = 2.0;
                        s = new;
                        if finished || s.chi2 == 0.0 {
                            return Some(done(s, it, true));
                        }
                        break;
                    }
                }
                if small_step(&s.q, db.as_slice(), tol) {
                    return Some(done(s, it, true));
                }
            }
            lambda *= nu;
            nu *= 2.0;
            if !lambda.is_finite() || lambda > 1e32 * max_diag {
                return Some(done(s, it, false));
            }
        }
    }
    Some(done(s, max_iter, false))
}

fn done(s: State, iterations: u32, converged: bool) -> OdrOutcome {
    OdrOutcome {
        q: s.q,
        delta: s.delta,
        chi2: s.chi2,
        iterations,
        converged,
    }
}
