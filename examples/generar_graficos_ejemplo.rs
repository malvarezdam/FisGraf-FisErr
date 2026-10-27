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

    for (name, project) in [("imanes", imanes(&excel)), ("pendulo", pendulo(&excel))] {
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
    p
}
