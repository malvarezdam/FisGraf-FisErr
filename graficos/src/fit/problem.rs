//! Problema de mínimos cuadrados ponderados sobre los parámetros libres.
//!
//! Los parámetros fijos se guardan aparte: los métodos solo ven el vector `q`
//! de parámetros libres. Los residuos son `√wᵢ·(yᵢ − f(xᵢ))`.

use nalgebra::{DMatrix, DVector};

use crate::model::Model;

pub(crate) struct Problem<'a> {
    pub model: &'a Model,
    pub x: &'a [f64],
    pub y: &'a [f64],
    /// √w por punto (1/σ, o 1 sin pesos).
    pub sw: Vec<f64>,
    /// Vector completo de parámetros; los fijos conservan su valor.
    pub full: Vec<f64>,
    /// Índices de los parámetros libres dentro de `full`.
    pub free: Vec<usize>,
    pub lower: Vec<f64>,
    pub upper: Vec<f64>,
}

/// Resultado común de los métodos iterativos.
pub(crate) struct Outcome {
    pub q: Vec<f64>,
    pub iterations: u32,
    pub converged: bool,
}

impl Problem<'_> {
    pub fn n(&self) -> usize {
        self.x.len()
    }

    pub fn m(&self) -> usize {
        self.free.len()
    }

    pub fn initial(&self) -> Vec<f64> {
        self.free.iter().map(|&i| self.full[i]).collect()
    }

    pub fn expand(&self, q: &[f64]) -> Vec<f64> {
        let mut p = self.full.clone();
        for (&i, &v) in self.free.iter().zip(q) {
            p[i] = v;
        }
        p
    }

    /// Proyecta `q` dentro de los límites.
    pub fn clamp(&self, q: &mut [f64]) {
        for (k, v) in q.iter_mut().enumerate() {
            *v = v.clamp(self.lower[k], self.upper[k]);
        }
    }

    /// Residuos ponderados; `None` si alguno no es finito.
    pub fn residuals(&self, q: &[f64]) -> Option<DVector<f64>> {
        let p = self.expand(q);
        let r = DVector::from_fn(self.n(), |i, _| self.sw[i] * (self.y[i] - self.model.eval(self.x[i], &p)));
        r.iter().all(|v| v.is_finite()).then_some(r)
    }

    /// χ² ponderado; infinito si el modelo no se puede evaluar.
    pub fn chi2(&self, q: &[f64]) -> f64 {
        self.residuals(q).map_or(f64::INFINITY, |r| r.norm_squared())
    }

    /// Jacobiano ponderado del modelo, `√wᵢ·∂f/∂qₖ`, de tamaño n × m.
    pub fn jacobian(&self, q: &[f64]) -> DMatrix<f64> {
        let p = self.expand(q);
        let mut grad = vec![0.0; p.len()];
        let mut j = DMatrix::zeros(self.n(), self.m());
        for i in 0..self.n() {
            self.model.gradient(self.x[i], &p, &mut grad);
            for (k, &idx) in self.free.iter().enumerate() {
                j[(i, k)] = self.sw[i] * grad[idx];
            }
        }
        j
    }
}

/// Criterio de convergencia compartido: paso relativo pequeño o χ² sin cambio relativo.
pub(crate) fn small_step(q: &[f64], step: &[f64], tol: f64) -> bool {
    q.iter().zip(step).all(|(v, d)| d.abs() <= tol * (v.abs() + tol))
}

pub(crate) fn small_reduction(old: f64, new: f64, tol: f64) -> bool {
    old - new <= tol * old || new <= f64::MIN_POSITIVE
}
