//! Validación contra los conjuntos certificados NIST StRD
//! (https://www.itl.nist.gov/div898/strd/). La precisión se mide con el
//! LRE (log relative error): número de cifras correctas, −log10(|x − c| / |c|).

use std::path::PathBuf;

use fisgraf::fit::{FitData, FitMethod, FitOptions, fit};
use fisgraf::model::Model;

struct Dataset {
    x: Vec<f64>,
    y: Vec<f64>,
    starts: [Vec<f64>; 2],
    certified: Vec<f64>,
    rss: Option<f64>,
}

fn load(name: &str) -> Dataset {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/nist/{name}.dat"));
    let text = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let nums = |s: &str| -> Vec<f64> { s.split_whitespace().filter_map(|t| t.parse().ok()).collect() };

    let mut starts = [Vec::new(), Vec::new()];
    let mut certified = Vec::new();
    let mut rss = None;
    for line in &lines {
        let t = line.trim_start();
        // No lineal: "b1 = start1 start2 certificado sd"
        if let Some(rest) = t.strip_prefix('b').filter(|r| r.starts_with(|c: char| c.is_ascii_digit()))
            && let Some((_, vals)) = rest.split_once('=')
        {
            let v = nums(vals);
            starts[0].push(v[0]);
            starts[1].push(v[1]);
            certified.push(v[2]);
        }
        // Lineal: "B0 certificado sd"
        if let Some(rest) = t.strip_prefix('B').filter(|r| r.starts_with(|c: char| c.is_ascii_digit())) {
            let v = nums(rest);
            if v.len() == 3 {
                certified.push(v[1]);
            }
        }
        if let Some(v) = t.strip_prefix("Residual Sum of Squares:") {
            rss = nums(v).first().copied();
        }
    }
    let header = lines.iter().rposition(|l| l.starts_with("Data:")).unwrap();
    let (mut x, mut y) = (Vec::new(), Vec::new());
    for l in &lines[header + 1..] {
        if let [yv, xv] = nums(l)[..] {
            y.push(yv);
            x.push(xv);
        }
    }
    if starts[0].is_empty() {
        starts = [vec![0.0; certified.len()], vec![0.0; certified.len()]];
    }
    Dataset { x, y, starts, certified, rss }
}

fn lre(value: f64, certified: f64) -> f64 {
    if value == certified {
        return 15.0;
    }
    let rel = if certified == 0.0 { value.abs() } else { ((value - certified) / certified).abs() };
    (-rel.log10()).min(15.0)
}

/// (nombre, modelo, dificultad)
const NONLINEAR: [(&str, &str, &str); 20] = [
    ("Misra1a", "b1*(1 - exp(-b2*x))", "baja"),
    ("Chwirut2", "exp(-b1*x)/(b2 + b3*x)", "baja"),
    ("DanWood", "b1*x^b2", "baja"),
    ("Misra1b", "b1*(1 - (1 + b2*x/2)^(-2))", "baja"),
    ("Kirby2", "(b1 + b2*x + b3*x^2)/(1 + b4*x + b5*x^2)", "media"),
    ("Hahn1", "(b1 + b2*x + b3*x^2 + b4*x^3)/(1 + b5*x + b6*x^2 + b7*x^3)", "media"),
    ("ENSO", "b1 + b2*cos(2*pi*x/12) + b3*sin(2*pi*x/12) + b5*cos(2*pi*x/b4) + b6*sin(2*pi*x/b4) + b8*cos(2*pi*x/b7) + b9*sin(2*pi*x/b7)", "media"),
    ("Gauss1", "b1*exp(-b2*x) + b3*exp(-(x - b4)^2/b5^2) + b6*exp(-(x - b7)^2/b8^2)", "baja"),
    ("Misra1c", "b1*(1 - (1 + 2*b2*x)^(-0.5))", "media"),
    ("Misra1d", "b1*b2*x*((1 + b2*x)^(-1))", "media"),
    ("Roszman1", "b1 - b2*x - atan(b3/(x - b4))/pi", "media"),
    ("Lanczos3", "b1*exp(-b2*x) + b3*exp(-b4*x) + b5*exp(-b6*x)", "baja"),
    ("Thurber", "(b1 + b2*x + b3*x^2 + b4*x^3)/(1 + b5*x + b6*x^2 + b7*x^3)", "alta"),
    ("MGH09", "b1*(x^2 + x*b2)/(x^2 + x*b3 + b4)", "alta"),
    ("MGH10", "b1*exp(b2/(x + b3))", "alta"),
    ("Eckerle4", "(b1/b2)*exp(-0.5*((x - b3)/b2)^2)", "alta"),
    ("Rat42", "b1/(1 + exp(b2 - b3*x))", "alta"),
    ("Rat43", "b1/((1 + exp(b2 - b3*x))^(1/b4))", "alta"),
    ("BoxBOD", "b1*(1 - exp(-b2*x))", "alta"),
    ("Bennett5", "b1*(b2 + x)^(-1/b3)", "alta"),
];

fn run(name: &str, expr: &str, method: FitMethod, start: usize) -> (f64, f64, bool) {
    let d = load(name);
    let params: Vec<String> = (1..=d.certified.len()).map(|i| format!("b{i}")).collect();
    let model = Model::new(expr, "x", &params, &[]).unwrap();
    let mut opts = FitOptions::new(method, d.starts[start].clone());
    opts.settings.max_iterations = 5000;
    opts.settings.tolerance = 1e-14;
    let data = FitData { x: &d.x, y: &d.y, x_err: None, y_err: None };
    match fit(&model, &data, &opts) {
        Ok(r) => {
            let p_lre = r.params.iter().zip(&d.certified).map(|(v, c)| lre(*v, *c)).fold(15.0, f64::min);
            let rss_lre = d.rss.map_or(15.0, |c| lre(r.chi2, c));
            (p_lre, rss_lre, r.converged)
        }
        Err(_) => (0.0, 0.0, false),
    }
}

/// Levenberg–Marquardt desde el punto de partida 2 (el más cercano):
/// exige al menos 6 cifras correctas en todos los parámetros y en el RSS.
#[test]
fn nonlinear_levenberg_marquardt_start2() {
    let mut failures = Vec::new();
    for (name, expr, level) in NONLINEAR {
        let (p, rss, conv) = run(name, expr, FitMethod::LevenbergMarquardt, 1);
        println!("{name:<9} {level:<6} parámetros LRE {p:5.1}  RSS LRE {rss:5.1}  convergió {conv}");
        if p < 6.0 || rss < 6.0 {
            failures.push(name);
        }
    }
    assert!(failures.is_empty(), "fallaron: {failures:?}");
}

/// Desde el punto de partida 1 (lejano). Los problemas de dificultad alta
/// fallan a menudo desde aquí en cualquier implementación; se exige que LM o
/// Nelder–Mead + LM resuelvan todos los de dificultad baja y media.
#[test]
fn nonlinear_start1() {
    let mut failures = Vec::new();
    for (name, expr, level) in NONLINEAR {
        let lm = run(name, expr, FitMethod::LevenbergMarquardt, 0);
        let nm = run(name, expr, FitMethod::NelderMead, 0);
        let best = lm.0.max(nm.0);
        println!("{name:<9} {level:<6} LM {:5.1}  NM+LM {:5.1}", lm.0, nm.0);
        if level != "alta" && best < 6.0 {
            failures.push(name);
        }
    }
    assert!(failures.is_empty(), "fallaron: {failures:?}");
}

#[test]
fn gauss_newton_start2_easy_problems() {
    for (name, expr, level) in NONLINEAR.iter().filter(|c| c.2 == "baja") {
        let (p, _, _) = run(name, expr, FitMethod::GaussNewton, 1);
        println!("{name:<9} {level:<6} GN LRE {p:5.1}");
        assert!(p >= 6.0, "{name}: LRE {p}");
    }
}

#[test]
fn linear_least_squares() {
    let poly5 = "B0 + B1*x + B2*x^2 + B3*x^3 + B4*x^4 + B5*x^5";
    let cases = [
        ("Norris", "B0 + B1*x", 9.0),
        ("Pontius", "B0 + B1*x + B2*x^2", 8.0),
        ("Wampler1", poly5, 8.0),
        ("Wampler2", poly5, 8.0),
        ("Wampler3", poly5, 7.0),
        ("Wampler4", poly5, 7.0),
        ("Wampler5", poly5, 6.0),
    ];
    for (name, expr, needed) in cases {
        let d = load(name);
        let params: Vec<String> = (0..d.certified.len()).map(|i| format!("B{i}")).collect();
        let model = Model::new(expr, "x", &params, &[]).unwrap();
        let opts = FitOptions::new(FitMethod::Linear, d.starts[0].clone());
        let data = FitData { x: &d.x, y: &d.y, x_err: None, y_err: None };
        let r = fit(&model, &data, &opts).unwrap();
        let p = r.params.iter().zip(&d.certified).map(|(v, c)| lre(*v, *c)).fold(15.0, f64::min);
        println!("{name:<9} lineal LRE {p:5.1}");
        assert!(p >= needed, "{name}: LRE {p} < {needed}");
    }
}

