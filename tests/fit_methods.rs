//! Pruebas de los métodos de ajuste con datos sintéticos y el Excel de ejemplo.

use std::path::PathBuf;

use fisgraf::config::{ErrorSpec, FitConfig, ModelConfig, ParameterConfig, SeriesConfig};
use fisgraf::data::{Workbook, load_series};
use fisgraf::fit::{FitData, FitError, FitMethod, FitOptions, build_model, fit};
use fisgraf::model::Model;

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Ruido determinista reproducible en [−1, 1].
fn noise(i: usize) -> f64 {
    ((i as f64 * 12.9898).sin() * 43758.5453).fract() * 2.0 - 1.0
}

fn line_data() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..12).map(|i| 1.0 + i as f64 * 0.75).collect();
    let sx: Vec<f64> = (0..12).map(|i| 0.05 + 0.01 * (i % 3) as f64).collect();
    let sy: Vec<f64> = (0..12).map(|i| 0.10 + 0.05 * (i % 4) as f64).collect();
    let y: Vec<f64> = x.iter().enumerate().map(|(i, x)| 2.0 + 1.5 * x + 0.3 * noise(i)).collect();
    (x, y, sx, sy)
}

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * b.abs().max(1e-300)
}

#[test]
fn all_methods_agree_on_a_line() {
    let (x, y, _, _) = line_data();
    let model = Model::new("a + b*x", "x", &names(&["a", "b"]), &[]).unwrap();
    let data = FitData { x: &x, y: &y, x_err: None, y_err: None };
    let reference = fit(&model, &data, &FitOptions::new(FitMethod::Linear, vec![0.0, 0.0])).unwrap();
    assert_eq!(reference.method, FitMethod::Linear);
    for method in [FitMethod::LevenbergMarquardt, FitMethod::GaussNewton, FitMethod::NelderMead, FitMethod::Auto] {
        let r = fit(&model, &data, &FitOptions::new(method, vec![1.0, 1.0])).unwrap();
        assert!(r.converged, "{method:?}");
        for k in 0..2 {
            assert!(close(r.params[k], reference.params[k], 1e-8), "{method:?}: {:?}", r.params);
        }
        assert_eq!(r.dof, 10);
    }
}

#[test]
fn weighted_line_matches_closed_form() {
    let (x, y, _, sy) = line_data();
    let model = Model::new("a + b*x", "x", &names(&["a", "b"]), &[]).unwrap();
    let data = FitData { x: &x, y: &y, x_err: None, y_err: Some(&sy) };
    let r = fit(&model, &data, &FitOptions::new(FitMethod::Auto, vec![0.0, 0.0])).unwrap();
    assert!(r.weighted);

    // Fórmulas cerradas de la recta ponderada.
    let w: Vec<f64> = sy.iter().map(|s| 1.0 / (s * s)).collect();
    let s: f64 = w.iter().sum();
    let sx: f64 = w.iter().zip(&x).map(|(w, x)| w * x).sum();
    let syy: f64 = w.iter().zip(&y).map(|(w, y)| w * y).sum();
    let sxx: f64 = w.iter().zip(&x).map(|(w, x)| w * x * x).sum();
    let sxy: f64 = (0..x.len()).map(|i| w[i] * x[i] * y[i]).sum();
    let det = s * sxx - sx * sx;
    let a = (sxx * syy - sx * sxy) / det;
    let b = (s * sxy - sx * syy) / det;
    assert!(close(r.params[0], a, 1e-10) && close(r.params[1], b, 1e-10));

    let chi2: f64 = (0..x.len()).map(|i| ((y[i] - a - b * x[i]) / sy[i]).powi(2)).sum();
    assert!(close(r.chi2, chi2, 1e-9));
    assert_eq!(r.sigma.as_deref(), Some(sy.as_slice()));
}

#[test]
fn nonlinear_recovers_parameters() {
    let x: Vec<f64> = (0..30).map(|i| i as f64 * 0.2).collect();
    let y: Vec<f64> = x.iter().enumerate().map(|(i, x)| 5.0 * (-x / 1.7).exp() * (3.0 * x + 0.4).cos() + 0.01 * noise(i)).collect();
    let model = Model::new("A*exp(-x/tau)*cos(w*x + phi)", "x", &names(&["A", "tau", "w", "phi"]), &[]).unwrap();
    let data = FitData { x: &x, y: &y, x_err: None, y_err: None };
    for method in [FitMethod::LevenbergMarquardt, FitMethod::NelderMead] {
        let r = fit(&model, &data, &FitOptions::new(method, vec![4.0, 1.5, 2.9, 0.3])).unwrap();
        let expected = [5.0, 1.7, 3.0, 0.4];
        for (got, want) in r.params.iter().zip(expected) {
            assert!(close(*got, want, 0.02), "{method:?}: {:?}", r.params);
        }
    }
}

#[test]
fn fixed_parameters_and_bounds() {
    let (x, y, _, _) = line_data();
    let model = Model::new("a + b*x", "x", &names(&["a", "b"]), &[]).unwrap();
    let data = FitData { x: &x, y: &y, x_err: None, y_err: None };

    let mut opts = FitOptions::new(FitMethod::LevenbergMarquardt, vec![2.5, 1.0]);
    opts.fixed[0] = true;
    let r = fit(&model, &data, &opts).unwrap();
    assert_eq!(r.params[0], 2.5);
    assert_eq!(r.free, [1]);
    assert_eq!(r.dof, 11);
    assert_eq!(r.jacobian.ncols(), 1);
    // Con a fija, b es la pendiente por mínimos cuadrados de y − a.
    let b: f64 = x.iter().zip(&y).map(|(x, y)| x * (y - 2.5)).sum::<f64>() / x.iter().map(|x| x * x).sum::<f64>();
    assert!(close(r.params[1], b, 1e-9));

    // Límite superior activo: la pendiente libre (≈1,5) queda en 1,4.
    let mut opts = FitOptions::new(FitMethod::LevenbergMarquardt, vec![0.0, 1.0]);
    opts.upper[1] = Some(1.4);
    let r = fit(&model, &data, &opts).unwrap();
    assert!(r.params[1] <= 1.4 + 1e-12 && r.params[1] > 1.39);

    // Auto con límites usa LM, no el método lineal.
    let mut opts = FitOptions::new(FitMethod::Auto, vec![0.0, 1.0]);
    opts.lower[0] = Some(-10.0);
    assert_eq!(fit(&model, &data, &opts).unwrap().method, FitMethod::LevenbergMarquardt);
}

/// Para una recta, ODR minimiza exactamente S(a, b) = Σ (y − a − bx)² / (σy² + b²σx²).
#[test]
fn odr_on_a_line_is_exact() {
    let (x, y, sx, sy) = line_data();
    let model = Model::new("a + b*x", "x", &names(&["a", "b"]), &[]).unwrap();
    let data = FitData { x: &x, y: &y, x_err: Some(&sx), y_err: Some(&sy) };
    let r = fit(&model, &data, &FitOptions::new(FitMethod::Odr, vec![0.0, 1.0])).unwrap();
    assert!(r.converged);
    let s = |a: f64, b: f64| -> f64 {
        (0..x.len()).map(|i| (y[i] - a - b * x[i]).powi(2) / (sy[i].powi(2) + b * b * sx[i].powi(2))).sum()
    };
    let (a, b) = (r.params[0], r.params[1]);
    assert!(close(r.chi2, s(a, b), 1e-8), "{} vs {}", r.chi2, s(a, b));
    for (da, db) in [(1e-4, 0.0), (-1e-4, 0.0), (0.0, 1e-5), (0.0, -1e-5)] {
        assert!(s(a + da, b + db) > s(a, b));
    }
    let shift = r.x_shift.as_ref().unwrap();
    assert_eq!(shift.len(), x.len());
    assert!(shift.iter().any(|d| *d != 0.0));

    // La varianza efectiva queda muy cerca: ignora que los pesos dependen de b,
    // lo que introduce un sesgo pequeño (≈0,1 % aquí) respecto de ODR.
    let ev = fit(&model, &data, &FitOptions::new(FitMethod::Auto, vec![0.0, 1.0])).unwrap();
    assert_eq!(ev.method, FitMethod::EffectiveVariance);
    assert!(close(ev.params[1], b, 3e-3), "{} vs {b}", ev.params[1]);
    assert!((ev.params[0] - a).abs() < 0.01);
}

#[test]
fn odr_nonlinear_matches_effective_variance_closely() {
    let x: Vec<f64> = (1..=15).map(|i| i as f64 * 0.4).collect();
    let sx = vec![0.02; 15];
    let sy = vec![0.05; 15];
    let y: Vec<f64> = x.iter().enumerate().map(|(i, x)| 3.0 * (1.0 - (-0.6 * x).exp()) + 0.04 * noise(i)).collect();
    let model = Model::new("A*(1 - exp(-k*x))", "x", &names(&["A", "k"]), &[]).unwrap();
    let data = FitData { x: &x, y: &y, x_err: Some(&sx), y_err: Some(&sy) };
    let odr = fit(&model, &data, &FitOptions::new(FitMethod::Odr, vec![2.0, 1.0])).unwrap();
    let ev = fit(&model, &data, &FitOptions::new(FitMethod::EffectiveVariance, vec![2.0, 1.0])).unwrap();
    assert!(odr.converged && ev.converged);
    for k in 0..2 {
        assert!(close(odr.params[k], ev.params[k], 5e-3), "{:?} vs {:?}", odr.params, ev.params);
    }
}

#[test]
fn readable_errors_and_warnings() {
    let (x, y, sx, mut sy) = line_data();
    let model = Model::new("a + b*x", "x", &names(&["a", "b"]), &[]).unwrap();
    let base = FitOptions::new(FitMethod::LevenbergMarquardt, vec![0.0, 1.0]);

    sy[3] = 0.0;
    let data = FitData { x: &x, y: &y, x_err: None, y_err: Some(&sy) };
    let err = fit(&model, &data, &base).unwrap_err();
    assert!(matches!(err, FitError::BadSigma { axis: "y", index: 3, .. }), "{err}");
    // Sin pesos, el σ nulo no molesta.
    let mut unweighted = base.clone();
    unweighted.settings.use_weights = false;
    assert!(!fit(&model, &data, &unweighted).unwrap().weighted);

    let data = FitData { x: &x[..1], y: &y[..1], x_err: None, y_err: None };
    assert_eq!(fit(&model, &data, &base).unwrap_err(), FitError::TooFewPoints { n: 1, m: 2 });

    let sqrt_model = Model::new("sqrt(x - c)", "x", &names(&["c"]), &[]).unwrap();
    let data = FitData { x: &x, y: &y, x_err: None, y_err: None };
    let err = fit(&sqrt_model, &data, &FitOptions::new(FitMethod::LevenbergMarquardt, vec![5.0])).unwrap_err();
    assert_eq!(err, FitError::NonFiniteStart { x: 1.0 });

    let nonlinear = Model::new("a*exp(b*x)", "x", &names(&["a", "b"]), &[]).unwrap();
    assert_eq!(
        fit(&nonlinear, &data, &FitOptions::new(FitMethod::Linear, vec![1.0, 0.1])).unwrap_err(),
        FitError::NotLinear
    );

    let mut bad = base.clone();
    bad.lower[1] = Some(2.0);
    assert!(matches!(fit(&model, &data, &bad).unwrap_err(), FitError::InitialOutOfBounds { .. }));

    // ODR sin errores en x: avisa y ajusta en y.
    let r = fit(&model, &data, &FitOptions::new(FitMethod::Odr, vec![0.0, 1.0])).unwrap();
    assert_eq!(r.method, FitMethod::Linear);
    assert!(r.warnings[0].contains("no hay errores en x"));

    // Errores en x con un método que no los usa: aviso.
    let data = FitData { x: &x, y: &y, x_err: Some(&sx), y_err: None };
    let r = fit(&model, &data, &base).unwrap();
    assert!(r.warnings.iter().any(|w| w.contains("errores de x no se usan")));
}

/// Péndulo del Excel de ejemplo: T = 2π √(L/g), con g = 9,79 m/s² al generarlo.
#[test]
fn pendulum_from_example_workbook() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/datos/ejemplo.xlsx");
    let book = Workbook::open(&path).unwrap();
    let series = SeriesConfig {
        x_range: "Pendulo!A2:A".into(),
        y_range: "Pendulo!B2:B".into(),
        x_error: ErrorSpec::Range("Pendulo!C2:C".into()),
        y_error: ErrorSpec::Range("Pendulo!D2:D".into()),
        ..Default::default()
    };
    let data = load_series(&book, &series).unwrap();

    let cfg = FitConfig {
        model: ModelConfig {
            expression: "2*pi*sqrt(L/g)".into(),
            variable: "L".into(),
            parameters: vec![ParameterConfig { name: "g".into(), initial: 10.0, lower: Some(1.0), ..Default::default() }],
            ..Default::default()
        },
        ..Default::default()
    };
    let model = build_model(&cfg.model).unwrap();
    for method in [FitMethod::Auto, FitMethod::Odr, FitMethod::LevenbergMarquardt] {
        let mut opts = FitOptions::from_config(&cfg);
        opts.method = method;
        let r = fit(&model, &FitData::from(&data), &opts).unwrap();
        assert!(r.converged);
        assert!((r.params[0] - 9.79).abs() < 0.05, "{method:?}: g = {}", r.params[0]);
        let chi2_red = r.reduced_chi2().unwrap();
        assert!(chi2_red > 0.05 && chi2_red < 5.0, "{method:?}: χ²/ν = {chi2_red}");
    }
}
