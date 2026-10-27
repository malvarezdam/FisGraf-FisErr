//! Pruebas de los métodos de incertidumbre, bandas y estadísticos.

use fisgraf::config::{BootstrapKind, CovarianceMode, UncertaintyConfig, UncertaintyMethod};
use fisgraf::fit::{FitData, FitMethod, FitOptions, FitResult, fit};
use fisgraf::model::Model;
use fisgraf::uncertainty::{
    SigFigs, UncertaintyError, UncertaintyResult, confidence_band, estimate, goodness_of_fit,
};

fn noise(i: usize) -> f64 {
    ((i as f64 * 12.9898).sin() * 43758.5453).fract() * 2.0 - 1.0
}

struct Line {
    x: Vec<f64>,
    y: Vec<f64>,
    sy: Vec<f64>,
    sx: Vec<f64>,
}

/// Recta con ruido gaussiano aproximado de σ conocida.
fn line() -> Line {
    let n = 25;
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.5).collect();
    let sy: Vec<f64> = (0..n).map(|i| 0.2 + 0.1 * (i % 3) as f64).collect();
    // Suma de tres uniformes ≈ normal; varianza de cada una = 1/3.
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let g = (noise(i) + noise(i + 100) + noise(i + 200)) / 1.0;
            1.0 + 0.8 * x[i] + sy[i] * g
        })
        .collect();
    Line { sx: vec![0.05; n], x, y, sy }
}

fn model() -> Model {
    Model::new("a + b*x", "x", &["a".to_string(), "b".to_string()], &[]).unwrap()
}

fn run(l: &Line, with_x: bool, cfg: &UncertaintyConfig) -> (FitResult, UncertaintyResult) {
    let m = model();
    let data = FitData { x: &l.x, y: &l.y, x_err: with_x.then_some(l.sx.as_slice()), y_err: Some(&l.sy) };
    let opts = FitOptions::new(FitMethod::Auto, vec![0.0, 1.0]);
    let r = fit(&m, &data, &opts).unwrap();
    let u = estimate(&m, &data, &opts, &r, cfg).unwrap();
    (r, u)
}

fn cfg(method: UncertaintyMethod) -> UncertaintyConfig {
    UncertaintyConfig { method, ..Default::default() }
}

#[test]
fn absolute_covariance_matches_closed_form() {
    let l = line();
    let c = UncertaintyConfig { covariance_mode: CovarianceMode::Absolute, ..Default::default() };
    let (_, u) = run(&l, false, &c);
    let w: Vec<f64> = l.sy.iter().map(|s| 1.0 / (s * s)).collect();
    let s: f64 = w.iter().sum();
    let sx: f64 = w.iter().zip(&l.x).map(|(w, x)| w * x).sum();
    let sxx: f64 = w.iter().zip(&l.x).map(|(w, x)| w * x * x).sum();
    let det = s * sxx - sx * sx;
    assert!((u.params[0].sigma - (sxx / det).sqrt()).abs() < 1e-12);
    assert!((u.params[1].sigma - (s / det).sqrt()).abs() < 1e-12);
    assert!((u.covariance[(0, 1)] - (-sx / det)).abs() < 1e-12);
    assert!((u.correlation[(0, 1)] + sx / (s * sxx).sqrt()).abs() < 1e-12);
    // Intervalo al 95 % con z = 1,96 (σ conocidas).
    let p = &u.params[1];
    assert!(((p.ci_high - p.value) / p.sigma - 1.959964).abs() < 1e-5);
}

#[test]
fn scaled_covariance_uses_student_t() {
    let l = line();
    let (r, u) = run(&l, false, &cfg(UncertaintyMethod::Covariance));
    let c_abs = UncertaintyConfig { covariance_mode: CovarianceMode::Absolute, ..Default::default() };
    let (_, abs) = run(&l, false, &c_abs);
    let ratio = u.params[1].sigma / abs.params[1].sigma;
    assert!((ratio - r.reduced_chi2().unwrap().sqrt()).abs() < 1e-12);
    // t de Student con 23 grados de libertad: 2,0687.
    assert!((u.coverage_factor - 2.068658).abs() < 1e-5);
}

#[test]
fn resampling_methods_agree_with_covariance() {
    let l = line();
    let (_, cov) = run(&l, false, &cfg(UncertaintyMethod::Covariance));
    let abs_cfg = UncertaintyConfig { covariance_mode: CovarianceMode::Absolute, ..Default::default() };
    let (_, abs) = run(&l, false, &abs_cfg);

    let cases = [
        (cfg(UncertaintyMethod::Bootstrap), &cov, 0.25),
        (UncertaintyConfig { bootstrap_kind: BootstrapKind::Pairs, ..cfg(UncertaintyMethod::Bootstrap) }, &cov, 0.35),
        (cfg(UncertaintyMethod::Jackknife), &cov, 0.40),
        // Monte Carlo propaga los σ de los datos: se compara con la covarianza absoluta.
        (cfg(UncertaintyMethod::MonteCarlo), &abs, 0.10),
    ];
    for (c, reference, tol) in cases {
        let (_, u) = run(&l, false, &c);
        for k in 0..2 {
            let rel = (u.params[k].sigma / reference.params[k].sigma - 1.0).abs();
            assert!(rel < tol, "{:?} {:?}: σ = {} vs {} ({rel:.2})", c.method, c.bootstrap_kind, u.params[k].sigma, reference.params[k].sigma);
            assert!(u.params[k].ci_low < u.params[k].value && u.params[k].value < u.params[k].ci_high);
        }
    }
}

#[test]
fn resampling_is_reproducible_with_seed() {
    let l = line();
    let c = UncertaintyConfig { samples: 300, ..cfg(UncertaintyMethod::Bootstrap) };
    let (_, a) = run(&l, false, &c);
    let (_, b) = run(&l, false, &c);
    assert_eq!(a.params, b.params);
    let other = UncertaintyConfig { seed: Some(7), ..c };
    let (_, d) = run(&l, false, &other);
    assert_ne!(a.params[1].sigma, d.params[1].sigma);
    assert_eq!(a.samples.as_ref().unwrap().len(), 300);
}

#[test]
fn monte_carlo_with_x_errors() {
    let l = line();
    let (r, u) = run(&l, true, &cfg(UncertaintyMethod::MonteCarlo));
    assert_eq!(r.method, FitMethod::EffectiveVariance);
    let abs_cfg = UncertaintyConfig { covariance_mode: CovarianceMode::Absolute, ..Default::default() };
    let (_, abs) = run(&l, true, &abs_cfg);
    let rel = (u.params[1].sigma / abs.params[1].sigma - 1.0).abs();
    assert!(rel < 0.1, "{rel}");
}

/// En un modelo lineal el perfil de χ² es una parábola exacta: coincide con la covarianza.
#[test]
fn profile_equals_covariance_for_linear_model() {
    let l = line();
    let (_, cov) = run(&l, false, &cfg(UncertaintyMethod::Covariance));
    let (_, prof) = run(&l, false, &cfg(UncertaintyMethod::ChiSquareProfile));
    for k in 0..2 {
        let (p, c) = (&prof.params[k], &cov.params[k]);
        assert!((p.minus / c.sigma - 1.0).abs() < 1e-4, "{} vs {}", p.minus, c.sigma);
        assert!((p.plus / c.sigma - 1.0).abs() < 1e-4);
        assert!((p.ci_high / c.ci_high - 1.0).abs() < 1e-5);
        assert!((p.ci_low / c.ci_low - 1.0).abs() < 1e-5);
    }
}

#[test]
fn profile_is_asymmetric_for_nonlinear_model() {
    let x: Vec<f64> = (1..=8).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().enumerate().map(|(i, x)| 10.0 * (-x / 3.0).exp() + 0.3 * noise(i)).collect();
    let m = Model::new("A*exp(-x/tau)", "x", &["A".to_string(), "tau".to_string()], &[]).unwrap();
    let data = FitData { x: &x, y: &y, x_err: None, y_err: None };
    let opts = FitOptions::new(FitMethod::LevenbergMarquardt, vec![8.0, 2.0]);
    let r = fit(&m, &data, &opts).unwrap();
    let u = estimate(&m, &data, &opts, &r, &cfg(UncertaintyMethod::ChiSquareProfile)).unwrap();
    let tau = &u.params[1];
    assert!(tau.plus > tau.minus * 1.02, "−{} +{}", tau.minus, tau.plus);
    assert!(u.warnings.is_empty(), "{:?}", u.warnings);
}

#[test]
fn bands() {
    let l = line();
    let m = model();
    let (r, u) = run(&l, false, &cfg(UncertaintyMethod::Covariance));
    let xs = [0.0, 6.0, 12.0];
    let band = confidence_band(&m, &r, &u, &xs);
    let (sa, sb, cab) = (u.covariance[(0, 0)], u.covariance[(1, 1)], u.covariance[(0, 1)]);
    for (x, (lo, hi)) in xs.iter().zip(&band) {
        let half = u.coverage_factor * (sa + x * x * sb + 2.0 * x * cab).sqrt();
        assert!(((hi - lo) / 2.0 - half).abs() < 1e-12);
    }
    // La banda es más angosta cerca del centro de los datos.
    assert!(band[1].1 - band[1].0 < band[0].1 - band[0].0);

    let (r, u) = run(&l, false, &cfg(UncertaintyMethod::Bootstrap));
    let band = confidence_band(&m, &r, &u, &xs);
    for (x, (lo, hi)) in xs.iter().zip(&band) {
        let f = r.params[0] + r.params[1] * x;
        assert!(*lo < f && f < *hi);
    }
}

#[test]
fn goodness_statistics() {
    let l = line();
    let (r, _) = run(&l, false, &cfg(UncertaintyMethod::Covariance));
    let g = goodness_of_fit(&r, &l.y);
    assert_eq!((g.n, g.n_free, g.dof), (25, 2, 23));
    assert!(g.r_squared > 0.95 && g.r_squared < 1.0, "{}", g.r_squared);
    assert!(g.adj_r_squared.unwrap() < g.r_squared);
    let p = g.p_value.unwrap();
    assert!(p > 0.0 && p < 1.0);
    assert!((g.aic - (r.chi2 + 4.0)).abs() < 1e-12);
    assert!((g.bic - (r.chi2 + 2.0 * 25f64.ln())).abs() < 1e-12);
    assert!(g.aicc.unwrap() > g.aic);
}

#[test]
fn errors() {
    let l = line();
    let m = model();
    let opts = FitOptions::new(FitMethod::Auto, vec![0.0, 1.0]);

    let bare = FitData { x: &l.x, y: &l.y, x_err: None, y_err: None };
    let r = fit(&m, &bare, &opts).unwrap();
    assert_eq!(
        estimate(&m, &bare, &opts, &r, &cfg(UncertaintyMethod::MonteCarlo)).unwrap_err(),
        UncertaintyError::NeedsErrors
    );
    let abs = UncertaintyConfig { covariance_mode: CovarianceMode::Absolute, ..Default::default() };
    assert!(estimate(&m, &bare, &opts, &r, &abs).unwrap().warnings[0].contains("χ²_red"));
    let bad = UncertaintyConfig { confidence_level: 1.0, ..Default::default() };
    assert_eq!(estimate(&m, &bare, &opts, &r, &bad).unwrap_err(), UncertaintyError::BadLevel);

    let two = FitData { x: &l.x[..2], y: &l.y[..2], x_err: None, y_err: None };
    let r = fit(&m, &two, &opts).unwrap();
    assert_eq!(
        estimate(&m, &two, &opts, &r, &UncertaintyConfig::default()).unwrap_err(),
        UncertaintyError::NoDegreesOfFreedom
    );
}

#[test]
fn fixed_parameters_and_rounding() {
    let l = line();
    let m = model();
    let data = FitData { x: &l.x, y: &l.y, x_err: None, y_err: Some(&l.sy) };
    let mut opts = FitOptions::new(FitMethod::Auto, vec![1.0, 1.0]);
    opts.fixed[0] = true;
    let r = fit(&m, &data, &opts).unwrap();
    let u = estimate(&m, &data, &opts, &r, &UncertaintyConfig::default()).unwrap();
    assert!(u.params[0].fixed && u.params[0].sigma == 0.0);
    assert_eq!(u.covariance.nrows(), 1);
    assert_eq!(u.params[0].rounded(SigFigs::Auto, None).plain(true), "1,00000");
    let b = u.params[1].rounded(SigFigs::Auto, None).plain(true);
    assert!(b.starts_with("0,") && b.contains(" ± 0,0"), "{b}");
}
