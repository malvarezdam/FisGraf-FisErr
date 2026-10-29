//! Métodos por re-muestreo: bootstrap (residuos o pares), Monte Carlo y jackknife.
//! Cada muestra se ajusta de nuevo partiendo de los parámetros óptimos.

use nalgebra::DMatrix;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use rand_distr::{Distribution, StandardNormal};

use crate::config::BootstrapKind;
use crate::fit::{FitData, FitOptions, FitResult, fit};
use crate::model::Model;

pub(crate) struct Resampler<'a> {
    pub model: &'a Model,
    pub data: FitData<'a>,
    /// Opciones con el método ya resuelto y los parámetros óptimos como inicio.
    pub opts: FitOptions,
    pub result: &'a FitResult,
}

impl Resampler<'_> {
    /// Ajusta datos alternativos y devuelve los parámetros libres.
    fn refit(&self, x: &[f64], y: &[f64], sx: Option<&[f64]>, sy: Option<&[f64]>) -> Option<Vec<f64>> {
        let data = FitData { x, y, x_err: sx, y_err: sy };
        let r = fit(self.model, &data, &self.opts).ok()?;
        r.converged.then(|| r.free.iter().map(|&i| r.params[i]).collect())
    }

    /// Generador reproducible para la muestra `k`, independiente del número de hilos.
    fn rng(seed: u64, k: usize) -> StdRng {
        StdRng::seed_from_u64(seed ^ (k as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }

    pub fn bootstrap(&self, kind: BootstrapKind, samples: usize, seed: u64) -> Vec<Vec<f64>> {
        let d = &self.data;
        let n = d.x.len();
        let m = self.result.free.len();
        let sigma: Vec<f64> = self.result.sigma.clone().unwrap_or_else(|| vec![1.0; n]);
        let fitted: Vec<f64> = d.y.iter().zip(&self.result.residuals).map(|(y, r)| y - r).collect();
        // Residuos estandarizados e inflados por √(n/ν) para no subestimar la dispersión.
        let inflate = if self.result.dof > 0 { (n as f64 / self.result.dof as f64).sqrt() } else { 1.0 };
        let z: Vec<f64> = self.result.residuals.iter().zip(&sigma).map(|(r, s)| r / s * inflate).collect();

        parallel(samples, |k| {
            let mut rng = Self::rng(seed, k);
            match kind {
                BootstrapKind::Residuals => {
                    let y: Vec<f64> = (0..n).map(|i| fitted[i] + sigma[i] * z[rng.random_range(0..n)]).collect();
                    self.refit(d.x, &y, d.x_err, d.y_err)
                }
                BootstrapKind::Pairs => {
                    let idx: Vec<usize> = (0..n).map(|_| rng.random_range(0..n)).collect();
                    let mut distinct = idx.clone();
                    distinct.sort_unstable();
                    distinct.dedup();
                    if distinct.len() < m {
                        return None;
                    }
                    let pick = |v: &[f64]| idx.iter().map(|&i| v[i]).collect::<Vec<_>>();
                    let (sx, sy) = (d.x_err.map(pick), d.y_err.map(pick));
                    self.refit(&pick(d.x), &pick(d.y), sx.as_deref(), sy.as_deref())
                }
            }
        })
    }

    /// Propaga las incertidumbres de los datos (GUM, Suplemento 1): cada muestra
    /// suma ruido normal con σx y σy a los valores medidos.
    pub fn monte_carlo(&self, samples: usize, seed: u64) -> Vec<Vec<f64>> {
        let d = &self.data;
        let n = d.x.len();
        parallel(samples, |k| {
            let mut rng = Self::rng(seed, k);
            let mut noisy = |v: &[f64], s: Option<&[f64]>| -> Vec<f64> {
                (0..n)
                    .map(|i| {
                        let e: f64 = StandardNormal.sample(&mut rng);
                        v[i] + s.map_or(0.0, |s| s[i]) * e
                    })
                    .collect()
            };
            let x = noisy(d.x, d.x_err);
            let y = noisy(d.y, d.y_err);
            self.refit(&x, &y, d.x_err, d.y_err)
        })
    }

    /// Ajustes dejando fuera un punto cada vez.
    pub fn jackknife(&self) -> Vec<Vec<f64>> {
        let d = &self.data;
        let n = d.x.len();
        parallel(n, |k| {
            let drop = |v: &[f64]| v.iter().enumerate().filter(|(i, _)| *i != k).map(|(_, v)| *v).collect::<Vec<_>>();
            let (sx, sy) = (d.x_err.map(drop), d.y_err.map(drop));
            self.refit(&drop(d.x), &drop(d.y), sx.as_deref(), sy.as_deref())
        })
    }
}

/// Ejecuta `f(k)` para k = 0..count en varios hilos y conserva el orden de k.
fn parallel<F>(count: usize, f: F) -> Vec<Vec<f64>>
where
    F: Fn(usize) -> Option<Vec<f64>> + Sync,
{
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(count).max(1);
    let mut results: Vec<(usize, Vec<f64>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let f = &f;
                scope.spawn(move || (t..count).step_by(threads).filter_map(|k| f(k).map(|v| (k, v))).collect::<Vec<_>>())
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().expect("hilo de re-muestreo")).collect()
    });
    results.sort_by_key(|(k, _)| *k);
    results.into_iter().map(|(_, v)| v).collect()
}

/// Media de cada columna.
fn mean(samples: &[Vec<f64>], m: usize) -> Vec<f64> {
    let n = samples.len() as f64;
    (0..m).map(|k| samples.iter().map(|s| s[k]).sum::<f64>() / n).collect()
}

/// Covarianza muestral (divisor N − 1).
pub(crate) fn sample_covariance(samples: &[Vec<f64>], m: usize) -> DMatrix<f64> {
    let mu = mean(samples, m);
    let n = samples.len() as f64;
    DMatrix::from_fn(m, m, |i, j| {
        samples.iter().map(|s| (s[i] - mu[i]) * (s[j] - mu[j])).sum::<f64>() / (n - 1.0)
    })
}

/// Percentil p ∈ [0, 1] con interpolación lineal.
pub(crate) fn percentile(values: &mut [f64], p: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    let pos = p * (values.len() - 1) as f64;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    values[lo] + (values[hi] - values[lo]) * (pos - lo as f64)
}
