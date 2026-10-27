//! Matriz de covarianza por linealización: C = (JᵀWJ)⁻¹ · escala.

use nalgebra::DMatrix;

/// (JᵀJ)⁻¹ de un jacobiano ya ponderado, por SVD de J: V·S⁻²·Vᵀ.
/// Si J no tiene rango completo se usa la pseudo-inversa y se informa el rango.
///
/// Las columnas se normalizan antes (J = Ĵ·D ⇒ (JᵀJ)⁻¹ = D⁻¹(ĴᵀĴ)⁻¹D⁻¹) para
/// no perder precisión cuando los parámetros tienen escalas muy distintas.
pub fn unscaled_covariance(j: &DMatrix<f64>) -> (DMatrix<f64>, usize) {
    let m = j.ncols();
    if m == 0 {
        return (DMatrix::zeros(0, 0), 0);
    }
    let norms: Vec<f64> = j.column_iter().map(|c| c.norm()).map(|n| if n > 0.0 { n } else { 1.0 }).collect();
    let mut scaled = j.clone();
    for (k, n) in norms.iter().enumerate() {
        scaled.column_mut(k).unscale_mut(*n);
    }
    let (cov, rank) = svd_inverse(scaled);
    (DMatrix::from_fn(m, m, |a, b| cov[(a, b)] / (norms[a] * norms[b])), rank)
}

fn svd_inverse(j: DMatrix<f64>) -> (DMatrix<f64>, usize) {
    let m = j.ncols();
    let svd = j.clone().svd(false, true);
    let v_t = svd.v_t.expect("SVD con V");
    let s_max = svd.singular_values.max();
    let eps = s_max * f64::EPSILON * j.nrows().max(m) as f64;
    let mut rank = 0;
    let mut cov = DMatrix::zeros(m, m);
    for (k, &s) in svd.singular_values.iter().enumerate() {
        if s > eps {
            rank += 1;
            let row = v_t.row(k);
            cov += (row.transpose() * row) / (s * s);
        }
    }
    (cov, rank)
}

/// Matriz de correlación ρᵢⱼ = Cᵢⱼ / (σᵢσⱼ).
pub fn correlation(cov: &DMatrix<f64>) -> DMatrix<f64> {
    let m = cov.nrows();
    DMatrix::from_fn(m, m, |i, j| {
        let d = (cov[(i, i)] * cov[(j, j)]).sqrt();
        if i == j {
            1.0
        } else if d > 0.0 {
            cov[(i, j)] / d
        } else {
            f64::NAN
        }
    })
}
