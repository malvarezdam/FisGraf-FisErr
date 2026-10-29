//! Intervalos por perfil de χ²: para cada parámetro θₖ se busca dónde
//! min_{otros} χ²(θₖ) = χ²_min + Δ. Da errores asimétricos en modelos muy no lineales.

use crate::fit::{FitData, FitOptions, FitResult, fit};
use crate::model::Model;

pub(crate) struct Profiler<'a> {
    pub model: &'a Model,
    pub data: FitData<'a>,
    pub opts: FitOptions,
    pub result: &'a FitResult,
}

/// Distancias (hacia abajo, hacia arriba) al cruce. `None` si no se alcanzó.
pub(crate) type Crossing = (Option<f64>, Option<f64>);

impl Profiler<'_> {
    /// χ²(θₖ = v) − χ²_min, ajustando los demás parámetros desde `start`.
    fn excess(&self, index: usize, v: f64, start: &[f64]) -> Option<(f64, Vec<f64>)> {
        let mut opts = self.opts.clone();
        opts.initial = start.to_vec();
        opts.initial[index] = v;
        opts.fixed[index] = true;
        // Un valor fijo fuera de los límites no es admisible.
        if opts.lower[index].is_some_and(|l| v < l) || opts.upper[index].is_some_and(|u| v > u) {
            return None;
        }
        let r = fit(self.model, &self.data, &opts).ok()?;
        r.converged.then_some((r.chi2 - self.result.chi2, r.params))
    }

    /// Busca a ambos lados del óptimo el punto donde el exceso de χ² vale `target`.
    pub fn crossings(&self, index: usize, target: f64, step0: f64) -> Crossing {
        let v0 = self.result.params[index];
        let side = |dir: f64| -> Option<f64> {
            let mut start = self.result.params.clone();
            let (mut inside, mut step) = (0.0, step0);
            let mut outside = None;
            for _ in 0..40 {
                let v = v0 + dir * step;
                match self.excess(index, v, &start) {
                    Some((ex, params)) if ex < target => {
                        inside = step;
                        start = params;
                        step *= 2.0;
                    }
                    _ => {
                        outside = Some(step);
                        break;
                    }
                }
            }
            let mut outside = outside?;
            // Bisección entre la distancia que queda dentro y la que queda fuera.
            for _ in 0..50 {
                if outside - inside <= 1e-7 * step0.max(f64::MIN_POSITIVE) {
                    break;
                }
                let mid = 0.5 * (inside + outside);
                match self.excess(index, v0 + dir * mid, &start) {
                    Some((ex, params)) if ex < target => {
                        inside = mid;
                        start = params;
                    }
                    _ => outside = mid,
                }
            }
            Some(0.5 * (inside + outside))
        };
        (side(-1.0), side(1.0))
    }
}
