//! Crea los proyectos de ejemplo (`examples/proyectos/*.fisgraf.json`) a partir
//! de `examples/datos/ejemplo.xlsx` y exporta sus gráficos a `examples/salida/`.
//!
//! Uso: `cargo run --example generar_graficos_ejemplo`

use std::path::Path;

use fisgraf::config::*;
use fisgraf::data::Workbook;
use fisgraf::pipeline::{build_scene, compute};
use fisgraf::render::export;

fn main() -> anyhow::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let projects = root.join("proyectos");
    std::fs::create_dir_all(&projects)?;
    let excel = root.join("datos").join("ejemplo.xlsx");

    let all = [
        ("imanes", imanes(&excel)),
        ("pendulo", pendulo(&excel)),
        ("hooke", hooke(&excel)),
        ("decaimiento", decaimiento(&excel)),
    ];
    for (name, project) in all {
        let path = projects.join(format!("{name}.{PROJECT_EXTENSION}"));
        project.save(&path)?;
        // Se vuelve a cargar para recorrer el mismo camino que la interfaz.
        let project = ProjectConfig::load(&path)?;
        let book = Workbook::open(project.excel_path.as_ref().expect("ruta del Excel"))?;
        let computed = compute(&project, Some(&book));
        let scene = build_scene(&project, &computed);
        for w in &scene.warnings {
            println!("aviso ({name}): {w}");
        }
        for run in &computed.fits {
            if let Ok(o) = &run.outcome
                && let Ok(u) = &o.uncertainty
            {
                for (p, pc) in u.params.iter().zip(o.model.params()) {
                    println!("{name}: {pc} = {}", p.rounded(fisgraf::uncertainty::SigFigs::Auto, None).plain(true));
                }
            }
        }
        let report = export(&project, &scene, &root.join("salida"))?;
        for f in report.files {
            println!("escrito {}", f.display());
        }
    }
    Ok(())
}

/// Réplica del gráfico de referencia: dos funciones teóricas y datos de Tracker.
fn imanes(excel: &Path) -> ProjectConfig {
    let mut p = ProjectConfig { excel_path: Some(excel.to_owned()), ..Default::default() };
    let id = p.add_series();
    let s = &mut p.series[0];
    s.id = id;
    s.label = "Tracker".into();
    s.x_range = "Imanes!A2:A5".into();
    s.y_range = "Imanes!B2:B5".into();
    s.y_error = ErrorSpec::Range("Imanes!C2:C5".into());
    s.x_error = ErrorSpec::Constant(0.05);
    s.color = Rgb(0x00, 0x72, 0xBD);
    s.marker = Marker::Square;
    s.marker_size_pt = 1.8;

    let t_teo = "0.969 + 0.0128*(x - 25.15)";
    p.references.push(ReferenceFunction {
        id: p.next_id(),
        label: "Modelo $T_{\\mathrm{teo}}$".into(),
        expression: t_teo.into(),
        line: LineStyle { color: Some(Rgb(0xD6, 0x27, 0x28)), dash: DashPattern::Dashed, width_pt: 1.2, opacity: 1.0 },
        ..Default::default()
    });
    p.references.push(ReferenceFunction {
        id: p.next_id(),
        label: "$0{,}987\\,T_{\\mathrm{teo}}$".into(),
        expression: format!("0.987*({t_teo})"),
        line: LineStyle { color: Some(Rgb::GRAY), dash: DashPattern::DenselyDotted, width_pt: 1.2, opacity: 1.0 },
        ..Default::default()
    });

    p.x_axis = AxisConfig {
        label: "Posición del par de imanes $x$ [\\unit{cm}]".into(),
        min: Some(25.15),
        max: Some(29.8),
        ticks: TickConfig { spacing: TickSpacing::Distance(1.0), ..Default::default() },
        number_format: NumberFormat { notation: Notation::Fixed, decimals: 0, ..Default::default() },
        ..Default::default()
    };
    p.y_axis = AxisConfig {
        label: "Periodo $T$ [\\unit{s}]".into(),
        min: Some(0.95),
        max: Some(1.02),
        ticks: TickConfig { spacing: TickSpacing::Distance(0.01), ..Default::default() },
        number_format: NumberFormat { notation: Notation::Fixed, decimals: 2, ..Default::default() },
        ..Default::default()
    };
    p.figure.width_cm = 9.0;
    p.figure.legend.position = LegendPosition::SouthEast;
    p.export = ExportConfig { file_stem: "imanes".into(), png: true, tex_standalone: true, tex_snippet: true, ..Default::default() };
    p
}

/// Péndulo: ajuste T = 2π√(L/g) con errores en x e y, banda, resultados y residuos.
fn pendulo(excel: &Path) -> ProjectConfig {
    let mut p = ProjectConfig { excel_path: Some(excel.to_owned()), ..Default::default() };
    let sid = p.add_series();
    let s = &mut p.series[0];
    s.label = "Mediciones".into();
    s.x_range = "Pendulo!A2:A".into();
    s.y_range = "Pendulo!B2:B".into();
    s.x_error = ErrorSpec::Range("Pendulo!C2:C".into());
    s.y_error = ErrorSpec::Range("Pendulo!D2:D".into());

    p.fits.push(FitConfig {
        id: p.next_id(),
        series_ids: vec![sid],
        label: "Ajuste $T = 2\\pi\\sqrt{L/g}$".into(),
        model: ModelConfig {
            expression: "2*pi*sqrt(L/g)".into(),
            variable: "L".into(),
            parameters: vec![ParameterConfig {
                name: "g".into(),
                unit: "m/s^2".into(),
                initial: 10.0,
                lower: Some(0.1),
                ..Default::default()
            }],
            latex_override: String::new(),
        },
        uncertainty: UncertaintyConfig { covariance_mode: CovarianceMode::Absolute, ..Default::default() },
        line: LineStyle { color: Some(Rgb(0xD6, 0x27, 0x28)), ..Default::default() },
        show_band: true,
        show_in_results_box: true,
        ..Default::default()
    });

    p.x_axis = AxisConfig {
        label: "Longitud $L$ [\\unit{m}]".into(),
        number_format: NumberFormat { notation: Notation::Fixed, decimals: 1, ..Default::default() },
        ..Default::default()
    };
    p.y_axis = AxisConfig {
        label: "Periodo $T$ [\\unit{s}]".into(),
        number_format: NumberFormat { notation: Notation::Fixed, decimals: 1, ..Default::default() },
        ..Default::default()
    };
    p.figure.width_cm = 9.0;
    p.figure.aspect = AspectRatio::R3x2;
    p.figure.legend.position = LegendPosition::SouthEast;
    p.figure.results_box = ResultsBoxConfig { show: true, position: LegendPosition::NorthWest };
    p.figure.residuals_panel = true;
    p.export = ExportConfig { file_stem: "pendulo".into(), png: true, svg: true, ..Default::default() };

    // Propagación de errores: g a partir de una sola medición de L y T.
    let id = p.next_id();
    p.propagations.push(PropagationConfig {
        id,
        name: "g".into(),
        unit: "m/s^2".into(),
        expression: "4*pi^2*L/T^2".into(),
        variables: vec![
            PropagationVariable {
                name: "L".into(),
                value: 1.000,
                sigma: 0.001 / (2.0 * 3f64.sqrt()),
                distribution: InputDistribution::Uniform,
                unit: "m".into(),
                ..Default::default()
            },
            PropagationVariable { name: "T".into(), value: 2.007, sigma: 0.005, unit: "s".into(), ..Default::default() },
        ],
        ..Default::default()
    });
    p
}

/// Ley de Hooke: Δx = g·m/k, con números escritos como texto en el Excel y
/// error de y dado por la resolución de la regla (1 mm).
fn hooke(excel: &Path) -> ProjectConfig {
    let mut p = ProjectConfig { excel_path: Some(excel.to_owned()), ..Default::default() };
    let sid = p.add_series();
    let s = &mut p.series[0];
    s.label = "Resorte".into();
    s.x_range = "Hooke!A2:A6".into();
    s.y_range = "Hooke!B2:B6".into();
    s.y_error = ErrorSpec::Resolution(0.001);
    s.marker = Marker::Diamond;
    s.marker_size_pt = 2.5;

    p.fits.push(FitConfig {
        id: p.next_id(),
        series_ids: vec![sid],
        label: r"$\Delta x = g\,m/k$".into(),
        model: ModelConfig {
            expression: "9.81*m/k".into(),
            variable: "m".into(),
            parameters: vec![ParameterConfig { name: "k".into(), unit: "N/m".into(), initial: 30.0, lower: Some(0.0), ..Default::default() }],
            latex_override: String::new(),
        },
        uncertainty: UncertaintyConfig { covariance_mode: CovarianceMode::Absolute, ..Default::default() },
        show_in_results_box: true,
        ..Default::default()
    });
    p.x_axis = AxisConfig {
        label: r"Masa $m$ [\unit{kg}]".into(),
        min: Some(0.0),
        max: Some(0.3),
        number_format: NumberFormat { notation: Notation::Fixed, decimals: 2, ..Default::default() },
        ..Default::default()
    };
    p.y_axis = AxisConfig {
        label: r"Elongación $\Delta x$ [\unit{m}]".into(),
        number_format: NumberFormat { notation: Notation::Fixed, decimals: 2, ..Default::default() },
        ..Default::default()
    };
    p.figure.font = FontChoice::Palatino;
    p.figure.results_box = ResultsBoxConfig { show: true, position: LegendPosition::NorthWest };
    p.figure.residuals_panel = true;
    p.export = ExportConfig { file_stem: "hooke".into(), png: true, ..Default::default() };
    p
}

/// Decaimiento radiactivo con fondo, eje y logarítmico y errores asimétricos
/// por perfil de χ².
fn decaimiento(excel: &Path) -> ProjectConfig {
    let mut p = ProjectConfig { excel_path: Some(excel.to_owned()), ..Default::default() };
    let sid = p.add_series();
    let s = &mut p.series[0];
    s.label = "Contador Geiger".into();
    s.x_range = "Decaimiento!A2:A".into();
    s.y_range = "Decaimiento!B2:B".into();
    s.y_error = ErrorSpec::Range("Decaimiento!C2:C".into());
    s.color = Rgb(0x2C, 0xA0, 0x2C);

    p.fits.push(FitConfig {
        id: p.next_id(),
        series_ids: vec![sid],
        label: r"$N_0\,e^{-t/\tau} + B$".into(),
        model: ModelConfig {
            expression: "N0*exp(-t/tau) + B".into(),
            variable: "t".into(),
            parameters: vec![
                ParameterConfig { name: "N0".into(), initial: 800.0, lower: Some(0.0), ..Default::default() },
                ParameterConfig { name: "tau".into(), unit: "s".into(), initial: 60.0, lower: Some(1.0), ..Default::default() },
                ParameterConfig { name: "B".into(), initial: 20.0, lower: Some(0.0), ..Default::default() },
            ],
            latex_override: String::new(),
        },
        method: FitMethod::LevenbergMarquardt,
        uncertainty: UncertaintyConfig {
            method: UncertaintyMethod::ChiSquareProfile,
            covariance_mode: CovarianceMode::Absolute,
            ..Default::default()
        },
        line: LineStyle { color: Some(Rgb(0x94, 0x67, 0xBD)), ..Default::default() },
        show_band: true,
        show_in_results_box: true,
        ..Default::default()
    });
    p.references.push(ReferenceFunction {
        id: p.next_id(),
        label: "Fondo $B = 50$".into(),
        expression: "50 + 0*t".into(),
        variable: "t".into(),
        ..Default::default()
    });
    p.x_axis = AxisConfig {
        label: r"Tiempo $t$ [\unit{s}]".into(),
        number_format: NumberFormat { notation: Notation::Fixed, decimals: 0, ..Default::default() },
        ..Default::default()
    };
    p.y_axis = AxisConfig {
        label: "Cuentas $N$".into(),
        scale: AxisScale::Log10,
        number_format: NumberFormat { notation: Notation::Fixed, decimals: 0, ..Default::default() },
        ..Default::default()
    };
    p.figure.aspect = AspectRatio::Golden;
    p.figure.legend.position = LegendPosition::NorthEast;
    p.figure.results_box = ResultsBoxConfig { show: true, position: LegendPosition::SouthWest };
    p.export = ExportConfig { file_stem: "decaimiento".into(), png: true, ..Default::default() };
    p
}
