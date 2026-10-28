//! Genera la carpeta `datos_de_prueba/` con mediciones simuladas de ocho
//! experimentos de física, cada una con su proyecto de FisGraf listo para abrir.
//!
//! Los datos salen de la ley física correspondiente más ruido gaussiano del
//! tamaño de los errores declarados (semilla fija: siempre los mismos datos).
//! La hoja «Notas» de cada Excel trae los valores reales para comprobar.
//!
//! Uso: `cargo run --example generar_datos_prueba`

use std::path::{Path, PathBuf};

use fisgraf::config::*;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand_distr::{Distribution, Normal};
use rust_xlsxwriter::{Format, Workbook};

/// Una tabla de mediciones: encabezados y columnas.
struct Table {
    headers: Vec<&'static str>,
    columns: Vec<Vec<f64>>,
}

struct Experiment {
    file: &'static str,
    sheet: &'static str,
    title: &'static str,
    description: &'static str,
    truth: &'static str,
    table: Table,
}

fn noise(seed: u64) -> impl FnMut(f64) -> f64 {
    let mut rng = StdRng::seed_from_u64(seed);
    let n = Normal::new(0.0, 1.0).expect("normal");
    move |sigma| sigma * n.sample(&mut rng)
}

fn round(v: f64, decimals: i32) -> f64 {
    let f = 10f64.powi(decimals);
    (v * f).round() / f
}

fn write_excel(dir: &Path, e: &Experiment) -> anyhow::Result<()> {
    let mut book = Workbook::new();
    let bold = Format::new().set_bold();
    let sheet = book.add_worksheet().set_name(e.sheet)?;
    for (c, h) in e.table.headers.iter().enumerate() {
        sheet.write_string_with_format(0, c as u16, *h, &bold)?;
        sheet.set_column_width(c as u16, 14)?;
    }
    for (c, col) in e.table.columns.iter().enumerate() {
        for (r, v) in col.iter().enumerate() {
            sheet.write_number(r as u32 + 1, c as u16, *v)?;
        }
    }
    let notes = book.add_worksheet().set_name("Notas")?;
    notes.set_column_width(0, 110)?;
    notes.write_string_with_format(0, 0, e.title, &bold)?;
    notes.write_string(2, 0, e.description)?;
    notes.write_string_with_format(4, 0, "Valores con que se generaron los datos:", &bold)?;
    notes.write_string(5, 0, e.truth)?;
    notes.write_string(7, 0, "Datos simulados con ruido gaussiano del tamaño de los errores indicados (FisGraf).")?;
    book.save(dir.join(e.file))?;
    Ok(())
}

// --- Ayudas para armar proyectos ---

fn base(dir: &Path, e: &Experiment) -> ProjectConfig {
    ProjectConfig { excel_path: Some(dir.join(e.file)), ..Default::default() }
}

fn add_series(p: &mut ProjectConfig, label: &str, x: &str, y: &str, ex: ErrorSpec, ey: ErrorSpec) -> u32 {
    let id = p.add_series();
    let s = p.series.last_mut().expect("recién creada");
    s.label = label.into();
    s.x_range = x.into();
    s.y_range = y.into();
    s.x_error = ex;
    s.y_error = ey;
    id
}

fn param(name: &str, initial: f64, unit: &str) -> ParameterConfig {
    ParameterConfig { name: name.into(), initial, unit: unit.into(), ..Default::default() }
}

fn fit(p: &ProjectConfig, series: Vec<u32>, label: &str, expr: &str, var: &str, params: Vec<ParameterConfig>) -> FitConfig {
    FitConfig {
        id: p.next_id(),
        series_ids: series,
        label: label.into(),
        model: ModelConfig { expression: expr.into(), variable: var.into(), parameters: params, latex_override: String::new() },
        show_in_results_box: true,
        ..Default::default()
    }
}

fn axis(label: &str, decimals: u8) -> AxisConfig {
    AxisConfig {
        label: label.into(),
        number_format: NumberFormat { notation: Notation::Fixed, decimals, ..Default::default() },
        ..Default::default()
    }
}

fn results(equation_lhs: &str) -> ResultsBoxItems {
    ResultsBoxItems { equation: true, equation_lhs: equation_lhs.into(), r_squared: true, ..Default::default() }
}

// --- Experimentos ---

fn caida_libre() -> Experiment {
    let mut n = noise(1);
    let g = 9.79;
    let t: Vec<f64> = (1..=10).map(|i| 0.05 * i as f64).collect();
    let tm: Vec<f64> = t.iter().map(|t| round(t + n(0.002), 4)).collect();
    let y: Vec<f64> = t.iter().map(|t| round(0.5 * g * t * t + 0.15 * t + n(0.003), 4)).collect();
    Experiment {
        file: "01_caida_libre.xlsx",
        sheet: "CaidaLibre",
        title: "Caída libre con fotopuertas",
        description: "Una esfera cae desde el reposo aproximado; se mide la distancia recorrida y el tiempo con fotopuertas. y = v0·t + g·t²/2.",
        truth: "g = 9,79 m/s², v0 = 0,15 m/s. σy = 3 mm (regla), σt = 2 ms (fotopuerta).",
        table: Table { headers: vec!["t [s]", "y [m]", "σt [s]", "σy [m]"], columns: vec![tm, y, vec![0.002; 10], vec![0.003; 10]] },
    }
}

fn ley_de_ohm() -> Experiment {
    let mut n = noise(2);
    let i: Vec<f64> = (1..=10).map(|k| 2.0 * k as f64).collect();
    let v = |r: f64, n: &mut dyn FnMut(f64) -> f64| -> Vec<f64> {
        i.iter().map(|i| { let v = r * i / 1000.0; round(v + n(0.005 * v + 0.01), 3) }).collect()
    };
    let v1 = v(100.0, &mut n);
    let v2 = v(220.0, &mut n);
    let s = |v: &[f64]| v.iter().map(|v| round(0.005 * v + 0.01, 4)).collect::<Vec<_>>();
    Experiment {
        file: "02_ley_de_ohm.xlsx",
        sheet: "Ohm",
        title: "Ley de Ohm con dos resistencias",
        description: "Se varía la corriente por dos resistencias y se mide la tensión con un multímetro (error 0,5 % + 1 dígito).",
        truth: "R1 = 100 Ω, R2 = 220 Ω (V = R·I, sin desplazamiento). σV = 0,5 % de la lectura + 0,01 V; σI = 0,1 mA.",
        table: Table {
            headers: vec!["I [mA]", "V (R1) [V]", "σV (R1) [V]", "V (R2) [V]", "σV (R2) [V]"],
            columns: vec![i.clone(), v1.clone(), s(&v1), v2.clone(), s(&v2)],
        },
    }
}

fn condensador() -> Experiment {
    let mut n = noise(3);
    let t: Vec<f64> = (0..=20).map(|k| 0.5 * k as f64).collect();
    let v: Vec<f64> = t.iter().map(|t| round(5.0 * (1.0 - (-t / 2.2).exp()) + n(0.02), 3)).collect();
    Experiment {
        file: "03_carga_condensador.xlsx",
        sheet: "RC",
        title: "Carga de un condensador (circuito RC)",
        description: "Se cierra el interruptor en t = 0 y se mide la tensión del condensador con un osciloscopio. V = ε·(1 − e^(−t/τ)), τ = RC.",
        truth: "ε = 5,00 V, τ = RC = 2,2 s (R = 10 kΩ, C = 220 µF). σV = 0,02 V.",
        table: Table { headers: vec!["t [s]", "V [V]"], columns: vec![t, v] },
    }
}

fn oscilador() -> Experiment {
    let mut n = noise(4);
    let t: Vec<f64> = (0..=50).map(|k| 0.2 * k as f64).collect();
    let x: Vec<f64> = t.iter().map(|t| round(4.0 * (-0.25 * t).exp() * (3.1 * t + 0.3).cos() + n(0.08), 3)).collect();
    Experiment {
        file: "04_oscilador_amortiguado.xlsx",
        sheet: "Oscilador",
        title: "Oscilador amortiguado (masa y resorte en aire)",
        description: "Posición de una masa colgada de un resorte, medida con un sensor de movimiento. x = A·e^(−γt)·cos(ωt + φ).",
        truth: "A = 4,0 cm, γ = 0,25 1/s, ω = 3,1 rad/s, φ = 0,3 rad. σx = 0,08 cm.",
        table: Table { headers: vec!["t [s]", "x [cm]"], columns: vec![t, x] },
    }
}

fn enfriamiento() -> Experiment {
    let mut n = noise(5);
    let t: Vec<f64> = (0..=15).map(|k| 2.0 * k as f64).collect();
    let temp: Vec<f64> = t.iter().map(|t| (22.0 + 63.0 * (-0.08 * t).exp() + n(0.3)).round()).collect();
    Experiment {
        file: "05_enfriamiento_newton.xlsx",
        sheet: "Enfriamiento",
        title: "Ley de enfriamiento de Newton",
        description: "Una taza de agua caliente se enfría en una sala a 22 °C; termómetro digital de resolución 1 °C. T = Ta + (T0 − Ta)·e^(−k·t).",
        truth: "Ta = 22 °C (fijo), T0 = 85 °C, k = 0,080 1/min. Resolución 1 °C → σ = 0,29 °C.",
        table: Table { headers: vec!["t [min]", "T [°C]"], columns: vec![t, temp] },
    }
}

fn resonancia() -> Experiment {
    let mut n = noise(6);
    let f: Vec<f64> = (0..20).map(|k| round(200.0 * 25f64.powf(k as f64 / 19.0), 0)).collect();
    let i: Vec<f64> = f
        .iter()
        .map(|f| {
            let ideal = 12.0 / (1.0 + 16.0 * (f / 1000.0 - 1000.0 / f).powi(2)).sqrt();
            round(ideal * (1.0 + n(0.02)), 3)
        })
        .collect();
    Experiment {
        file: "06_resonancia_rlc.xlsx",
        sheet: "RLC",
        title: "Resonancia de un circuito RLC serie",
        description: "Barrido de frecuencia con un generador de funciones; amperímetro con error del 2 %. I = I0 / √(1 + Q²(f/f0 − f0/f)²).",
        truth: "I0 = 12 mA, f0 = 1000 Hz, Q = 4. σI = 2 % de la lectura.",
        table: Table { headers: vec!["f [Hz]", "I [mA]"], columns: vec![f, i] },
    }
}

fn fotoelectrico() -> Experiment {
    let mut n = noise(7);
    // Líneas del mercurio: 578, 546, 436, 405 y 365 nm.
    let nu = vec![5.19, 5.49, 6.88, 7.41, 8.21];
    let v: Vec<f64> = nu.iter().map(|nu| round(0.4136 * nu - 2.0 + n(0.02), 3)).collect();
    Experiment {
        file: "07_efecto_fotoelectrico.xlsx",
        sheet: "Fotoelectrico",
        title: "Efecto fotoeléctrico (constante de Planck)",
        description: "Potencial de frenado para cinco líneas del mercurio. V0 = (h/e)·ν − W/e. Con la pendiente se obtiene h (ver pestaña Propagación).",
        truth: "h/e = 4,136·10⁻¹⁵ V·s (0,4136 V por 10¹⁴ Hz), W = 2,0 eV. σV0 = 0,02 V.",
        table: Table { headers: vec!["ν [10^14 Hz]", "V0 [V]", "σV0 [V]"], columns: vec![nu, v, vec![0.02; 5]] },
    }
}

fn boyle() -> Experiment {
    let mut n = noise(8);
    let v: Vec<f64> = (0..=10).map(|k| 10.0 + 2.0 * k as f64).collect();
    let p: Vec<f64> = v.iter().map(|v| round(2452.6 / v + n(1.0), 1)).collect();
    Experiment {
        file: "08_ley_de_boyle.xlsx",
        sheet: "Boyle",
        title: "Ley de Boyle (jeringa con manómetro)",
        description: "Se comprime aire en una jeringa a temperatura constante. P = nRT/V. Con el ajuste P = a/V se obtiene n (ver pestaña Propagación).",
        truth: "n = 1,00 mmol, T = 295 K → nRT = 2452,6 kPa·mL. σP = 1 kPa; σV: resolución 0,5 mL.",
        table: Table { headers: vec!["V [mL]", "P [kPa]"], columns: vec![v, p] },
    }
}

// --- Proyectos ---

fn proyecto(dir: &Path, e: &Experiment) -> ProjectConfig {
    let s = e.sheet;
    let r = |c: &str| format!("{s}!{c}2:{c}");
    let mut p = base(dir, e);
    p.export.file_stem = e.file.trim_end_matches(".xlsx").to_owned();
    p.export.png = true;
    match e.sheet {
        "CaidaLibre" => {
            let id = add_series(&mut p, "Fotopuertas", &r("A"), &r("B"), ErrorSpec::Range(r("C")), ErrorSpec::Range(r("D")));
            let mut f = fit(&p, vec![id], "$y = v_0 t + \\tfrac{1}{2} g t^2$", "v0*t + 0.5*g*t^2", "t", vec![param("v0", 0.0, "m/s"), param("g", 9.0, "m/s^2")]);
            f.model.parameters[0].latex = "v_0".into();
            f.results = results("y");
            f.results.chi2_red = true;
            f.legend_r2 = true;
            p.fits.push(f);
            p.x_axis = axis("Tiempo $t$ [\\unit{s}]", 2);
            p.y_axis = axis("Distancia $y$ [\\unit{m}]", 2);
            p.figure.results_box = ResultsBoxConfig { show: true, position: LegendPosition::NorthWest };
            p.figure.residuals_panel = true;
        }
        "Ohm" => {
            let a = add_series(&mut p, "$R_1$", &r("A"), &r("B"), ErrorSpec::Constant(0.1), ErrorSpec::Range(r("C")));
            let b = add_series(&mut p, "$R_2$", &r("A"), &r("D"), ErrorSpec::Constant(0.1), ErrorSpec::Range(r("E")));
            p.series[0].marker = Marker::Square;
            p.series[1].marker = Marker::Triangle;
            // V = R·I por el origen (I en mA).
            let mut f = fit(&p, vec![a, b], "$V = RI$", "R*I/1000", "I", vec![param("R", 150.0, "\\ohm")]);
            f.results = ResultsBoxItems { title: true, r_squared: true, chi2_red: false, ..Default::default() };
            f.legend_r2 = true;
            f.line.dash = DashPattern::Dashed;
            p.fits.push(f);
            p.figure.legend.position = LegendPosition::Below;
            p.figure.legend.columns = 2;
            p.x_axis = axis("Corriente $I$ [\\unit{mA}]", 0);
            p.y_axis = axis("Tensión $V$ [\\unit{V}]", 1);
            p.x_axis.min = Some(0.0);
            p.y_axis.min = Some(0.0);
            p.figure.font = FontChoice::Times;
            p.figure.results_box = ResultsBoxConfig { show: true, position: LegendPosition::NorthWest };
        }
        "RC" => {
            let id = add_series(&mut p, "Osciloscopio", &r("A"), &r("B"), ErrorSpec::None, ErrorSpec::Constant(0.02));
            let mut f = fit(&p, vec![id], "Ajuste exponencial", "eps*(1 - exp(-t/tau))", "t", vec![param("eps", 4.0, "V"), param("tau", 1.0, "s")]);
            f.model.parameters[0].latex = "\\varepsilon".into();
            f.uncertainty.method = UncertaintyMethod::Bootstrap;
            f.show_band = true;
            f.results.r_squared = true;
            f.results.points = true;
            p.fits.push(f);
            p.references.push(ReferenceFunction {
                id: p.next_id(),
                label: "$\\varepsilon = 5\\,\\unit{V}$".into(),
                expression: "5 + 0*t".into(),
                variable: "t".into(),
                ..Default::default()
            });
            p.x_axis = axis("Tiempo $t$ [\\unit{s}]", 0);
            p.y_axis = axis("Tensión $V_C$ [\\unit{V}]", 1);
            p.figure.results_box = ResultsBoxConfig { show: true, position: LegendPosition::SouthEast };
            p.figure.legend.position = LegendPosition::Custom { x: 0.97, y: 0.45 };
            p.figure.aspect = AspectRatio::R3x2;
        }
        "Oscilador" => {
            let id = add_series(&mut p, "Sensor de movimiento", &r("A"), &r("B"), ErrorSpec::None, ErrorSpec::Constant(0.08));
            p.series[0].marker_size_pt = 1.5;
            p.series[0].error_cap_pt = 1.0;
            let mut f = fit(
                &p,
                vec![id],
                "Ajuste",
                "A*exp(-gamma*t)*cos(w*t + phi)",
                "t",
                vec![param("A", 3.5, "cm"), param("gamma", 0.2, "1/s"), param("w", 3.0, "rad/s"), param("phi", 0.2, "rad")],
            );
            f.method = FitMethod::NelderMead;
            f.model.parameters[2].latex = "\\omega".into();
            f.samples = 600;
            f.results.chi2_red = true;
            f.results.points = true;
            p.fits.push(f);
            for (sign, label) in [("", "Envolvente $\\pm A e^{-\\gamma t}$"), ("-", "")] {
                p.references.push(ReferenceFunction {
                    id: p.next_id(),
                    label: label.into(),
                    show_in_legend: !label.is_empty(),
                    expression: format!("{sign}4*exp(-0.25*t)"),
                    variable: "t".into(),
                    line: LineStyle { color: Some(Rgb::GRAY), dash: DashPattern::Dotted, width_pt: 1.0, opacity: 1.0 },
                    ..Default::default()
                });
            }
            p.x_axis = axis("Tiempo $t$ [\\unit{s}]", 0);
            p.y_axis = axis("Posición $x$ [\\unit{cm}]", 0);
            p.x_axis.ticks.minor_per_major = 1;
            p.figure.width_cm = 14.0;
            p.figure.aspect = AspectRatio::R16x9;
            p.figure.grid.minor = true;
            p.figure.legend.position = LegendPosition::NorthEast;
            p.figure.results_box = ResultsBoxConfig { show: true, position: LegendPosition::SouthEast };
        }
        "Enfriamiento" => {
            let id = add_series(&mut p, "Termómetro", &r("A"), &r("B"), ErrorSpec::None, ErrorSpec::Resolution(1.0));
            p.series[0].color = Rgb(0xD6, 0x27, 0x28);
            p.series[0].marker = Marker::Diamond;
            let mut params = vec![param("Ta", 22.0, "\\degreeCelsius"), param("T0", 80.0, "\\degreeCelsius"), param("k", 0.1, "1/min")];
            params[0].fixed = true;
            params[0].latex = "T_a".into();
            params[1].latex = "T_0".into();
            let mut f = fit(&p, vec![id], "$T_a + (T_0 - T_a)\\,e^{-kt}$", "Ta + (T0 - Ta)*exp(-k*t)", "t", params);
            f.uncertainty.method = UncertaintyMethod::ChiSquareProfile;
            f.uncertainty.covariance_mode = CovarianceMode::Absolute;
            f.line.color = Some(Rgb(0x1F, 0x77, 0xB4));
            f.results = results("T");
            f.results.chi2_red = true;
            p.fits.push(f);
            p.x_axis = axis("Tiempo $t$ [\\unit{min}]", 0);
            p.y_axis = axis("Temperatura $T$ [\\unit{\\degreeCelsius}]", 0);
            p.figure.font = FontChoice::Palatino;
            p.figure.results_box = ResultsBoxConfig { show: true, position: LegendPosition::NorthEast };
            p.figure.legend.position = LegendPosition::SouthWest;
        }
        "RLC" => {
            let id = add_series(&mut p, "Amperímetro", &r("A"), &r("B"), ErrorSpec::None, ErrorSpec::Percent(2.0));
            let mut f = fit(
                &p,
                vec![id],
                "Curva de resonancia",
                "I0/sqrt(1 + Q^2*(f/f0 - f0/f)^2)",
                "f",
                vec![param("I0", 10.0, "mA"), param("Q", 3.0, ""), param("f0", 900.0, "Hz")],
            );
            f.model.parameters[0].latex = "I_0".into();
            f.model.parameters[2].latex = "f_0".into();
            f.samples = 400;
            f.results.r_squared = true;
            f.legend_r2 = false;
            p.fits.push(f);
            p.x_axis = AxisConfig {
                label: "Frecuencia $f$ [\\unit{Hz}]".into(),
                scale: AxisScale::Log10,
                ticks: TickConfig { spacing: TickSpacing::Manual(vec![200.0, 500.0, 1000.0, 2000.0, 5000.0]), ..Default::default() },
                number_format: NumberFormat { notation: Notation::Fixed, decimals: 0, ..Default::default() },
                ..Default::default()
            };
            p.y_axis = axis("Corriente $I$ [\\unit{mA}]", 0);
            p.figure.results_box = ResultsBoxConfig { show: true, position: LegendPosition::NorthWest };
            p.figure.legend.position = LegendPosition::NorthEast;
            p.figure.font = FontChoice::Charter;
        }
        "Fotoelectrico" => {
            let id = add_series(&mut p, "Líneas del Hg", &r("A"), &r("B"), ErrorSpec::None, ErrorSpec::Range(r("C")));
            p.series[0].color = Rgb(0x94, 0x67, 0xBD);
            let mut f = fit(&p, vec![id], "Ajuste lineal", "a*nu - b", "nu", vec![param("a", 0.4, "V"), param("b", 2.0, "V")]);
            f.model.parameters[0].latex = "h/e".into();
            f.model.parameters[1].latex = "W/e".into();
            f.results = ResultsBoxItems { r_squared: true, adj_r_squared: true, p_value: true, ..results("V_0") };
            f.show_band = true;
            p.fits.push(f);
            p.x_axis = axis("Frecuencia $\\nu$ [$10^{14}$\\,\\unit{Hz}]", 1);
            p.y_axis = axis("Potencial de frenado $V_0$ [\\unit{V}]", 1);
            p.figure.results_box = ResultsBoxConfig { show: true, position: LegendPosition::NorthWest };
            // h = (h/e)·e, con h/e de la pendiente (en V por 10^14 Hz).
            let pid = p.next_id();
            p.propagations.push(PropagationConfig {
                id: pid,
                name: "h".into(),
                unit: "J.s".into(),
                expression: "a*e0*1e-14".into(),
                variables: vec![
                    PropagationVariable { name: "a".into(), value: 0.4173, sigma: 0.0031, latex: "h/e".into(), unit: "V/(10^14 Hz)".into(), ..Default::default() },
                    PropagationVariable { name: "e0".into(), value: 1.602176634e-19, constant: true, latex: "e".into(), unit: "C".into(), ..Default::default() },
                ],
                ..Default::default()
            });
        }
        _ => {
            // Boyle
            let id = add_series(&mut p, "Jeringa", &r("A"), &r("B"), ErrorSpec::Resolution(0.5), ErrorSpec::Constant(1.0));
            let mut f = fit(&p, vec![id], "$P = a/V$", "a/V", "V", vec![param("a", 2000.0, "kPa.mL")]);
            f.method = FitMethod::Odr;
            f.results = ResultsBoxItems { r_squared: true, chi2_red: true, residual_std: true, ..results("P") };
            p.fits.push(f);
            p.x_axis = axis("Volumen $V$ [\\unit{mL}]", 0);
            p.y_axis = axis("Presión $P$ [\\unit{kPa}]", 0);
            p.figure.residuals_panel = true;
            p.figure.results_box = ResultsBoxConfig { show: true, position: LegendPosition::NorthEast };
            p.figure.legend.position = LegendPosition::SouthWest;
            p.figure.font = FontChoice::FiraSans;
            // n = a/(R·T): a en kPa·mL = mJ.
            let pid = p.next_id();
            p.propagations.push(PropagationConfig {
                id: pid,
                name: "n".into(),
                unit: "mol".into(),
                expression: "a*1e-3/(R*T)".into(),
                variables: vec![
                    PropagationVariable { name: "a".into(), value: 2446.0, sigma: 8.0, unit: "kPa.mL".into(), ..Default::default() },
                    PropagationVariable { name: "R".into(), value: 8.314462618, constant: true, unit: "J/(mol.K)".into(), ..Default::default() },
                    PropagationVariable {
                        name: "T".into(),
                        value: 295.0,
                        sigma: 0.5,
                        distribution: InputDistribution::Uniform,
                        unit: "K".into(),
                        ..Default::default()
                    },
                ],
                method: PropagationMethod::MonteCarlo,
                ..Default::default()
            });
        }
    }
    p
}

fn main() -> anyhow::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("datos_de_prueba");
    let projects = root.join("proyectos");
    std::fs::create_dir_all(&projects)?;
    let experiments = [
        caida_libre(),
        ley_de_ohm(),
        condensador(),
        oscilador(),
        enfriamiento(),
        resonancia(),
        fotoelectrico(),
        boyle(),
    ];
    for e in &experiments {
        write_excel(&root, e)?;
        let project = proyecto(&root, e);
        let path = projects.join(format!("{}.{PROJECT_EXTENSION}", e.file.trim_end_matches(".xlsx")));
        project.save(&path)?;
        println!("{} → {}", e.file, path.file_name().expect("nombre").to_string_lossy());

        // Gráfico de referencia, cargando el proyecto como lo haría la interfaz.
        let project = ProjectConfig::load(&path)?;
        let book = fisgraf::data::Workbook::open(project.excel_path.as_ref().expect("Excel"))?;
        let computed = fisgraf::pipeline::compute(&project, Some(&book));
        let scene = fisgraf::pipeline::build_scene(&project, &computed);
        for w in &scene.warnings {
            println!("   aviso: {w}");
        }
        for run in &computed.fits {
            if let Ok(o) = &run.outcome
                && let Ok(u) = &o.uncertainty
            {
                let text: Vec<String> = o
                    .model
                    .params()
                    .iter()
                    .zip(&u.params)
                    .map(|(n, p)| format!("{n} = {}", p.rounded(fisgraf::uncertainty::SigFigs::Auto, None).plain(true)))
                    .collect();
                println!("   {}  (R² = {:.5})", text.join(", "), o.goodness.r_squared);
            }
        }
        fisgraf::render::export(&project, &scene, &root.join("graficos"))?;
    }
    Ok(())
}
