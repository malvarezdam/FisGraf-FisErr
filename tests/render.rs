//! Pruebas del generador pgfplots y de la compilación real con LaTeX.
//! Las que compilan se saltan si no hay motor LaTeX instalado.

use std::path::PathBuf;

use fisgraf::config::*;
use fisgraf::data::Workbook;
use fisgraf::pipeline::{build_scene, compute};
use fisgraf::render::compile::{compile_pdf, find_program};
use fisgraf::render::export::COMPILE_TIMEOUT;
use fisgraf::render::{RenderError, Scene, export, generate};

fn example(name: &str) -> (ProjectConfig, Scene) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("examples/proyectos/{name}.fisgraf.json"));
    let project = ProjectConfig::load(&path).unwrap();
    let book = Workbook::open(project.excel_path.as_ref().unwrap()).unwrap();
    let scene = build_scene(&project, &compute(&project, Some(&book)));
    (project, scene)
}

fn has_latex(engine: LatexEngine) -> bool {
    let name = fisgraf::render::compile::engine_program(engine);
    let ok = find_program(name).is_some();
    if !ok {
        println!("{name} no está instalado: se omite la prueba");
    }
    ok
}

fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fisgraf_test_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn generated_tex_contains_configuration() {
    let (mut p, scene) = example("imanes");
    let doc = generate(&p, &scene);
    for needle in [
        "\\documentclass[border=4pt]{standalone}",
        "\\sisetup{output-decimal-marker={,}}",
        "xmin=25.15",
        "xmax=29.8",
        "ymin=0.95",
        "xtick distance=1",
        "fixed, precision=2, fixed zerofill, use comma",
        "grid=major",
        "anchor=south east",
        "\\definecolor{fg0072BD}{HTML}{0072BD}",
        "mark=square*",
        "x dir=both, x explicit, y dir=both, y explicit",
        "\\addlegendentry{Modelo $T_{\\mathrm{teo}}$}",
        "dashed",
        "densely dotted",
        "+- (5e-2,3e-3)",
    ] {
        assert!(doc.full.contains(needle), "falta «{needle}»");
    }
    assert!(!doc.full.contains("fillbetween"));
    assert!(doc.full.ends_with("\\end{document}\n"));
    assert!(doc.picture.starts_with("\\begin{tikzpicture}"));

    // Cambios de configuración que deben reflejarse.
    p.x_axis.scale = AxisScale::Log10;
    p.x_axis.ticks.spacing = TickSpacing::Manual(vec![26.0, 28.5]);
    p.x_axis.ticks.minor_per_major = 4;
    p.x_axis.ticks.direction = TickDirection::Out;
    p.figure.grid.minor = true;
    p.figure.title = "Título $\\alpha$".into();
    p.figure.legend.position = LegendPosition::Below;
    p.figure.legend.frame = false;
    p.y_axis.number_format = NumberFormat { notation: Notation::Scientific, decimals: 1, decimal_comma: false, ..Default::default() };
    p.figure.font = FontChoice::Times;
    p.figure.engine = LatexEngine::PdfLatex;
    let doc = generate(&p, &scene);
    for needle in [
        "xmode=log",
        "xtick={26,28.5}",
        "minor x tick num=4",
        "xtick align=outside",
        "grid=both",
        "title={Título $\\alpha$}",
        "legend columns=-1",
        "draw=none",
        "sci, precision=1",
        "\\usepackage{newtxtext,newtxmath}",
        "\\usepackage[T1]{fontenc}",
    ] {
        assert!(doc.full.contains(needle), "falta «{needle}»");
    }
    assert!(!doc.full.contains("output-decimal-marker"));
}

#[test]
fn pendulum_scene_has_fit_band_box_and_residuals() {
    let (p, scene) = example("pendulo");
    assert_eq!(scene.curves.len(), 1);
    assert!(scene.curves[0].band.is_some());
    assert_eq!(scene.residuals.len(), 1);
    assert!(scene.results_box[0].starts_with("$g = \\left(9{,}78"), "{:?}", scene.results_box);
    assert!(scene.results_box[0].ends_with("\\,\\unit{m/s^2}"));
    assert!(scene.warnings.is_empty(), "{:?}", scene.warnings);
    let doc = generate(&p, &scene);
    for needle in ["\\usepgfplotslibrary{fillbetween}", "fill between[of=band0lo and band0hi]", "ylabel={Residuos}", "xticklabels={}", "at={(main.below south west)}"] {
        assert!(doc.full.contains(needle), "falta «{needle}»");
    }
}

#[test]
fn hidden_legend_entries_use_forget_plot() {
    let (mut p, _) = example("imanes");
    p.references[1].show_in_legend = false;
    p.figure.legend.show = true;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/datos/ejemplo.xlsx");
    let book = Workbook::open(&path).unwrap();
    let scene = build_scene(&p, &compute(&p, Some(&book)));
    let doc = generate(&p, &scene);
    assert_eq!(doc.full.matches("\\addlegendentry").count(), 2);
    assert!(doc.full.contains("densely dotted, no marks, forget plot"));
}

#[test]
fn export_all_formats() {
    if !has_latex(LatexEngine::LuaLatex) {
        return;
    }
    let (mut p, scene) = example("pendulo");
    p.export = ExportConfig { file_stem: "prueba".into(), pdf: true, png: true, png_dpi: 100, svg: true, tex_standalone: true, tex_snippet: true, output_dir: None };
    let dir = temp_dir("export");
    let report = export(&p, &scene, &dir).unwrap();
    let names: Vec<String> = report.files.iter().map(|f| f.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["prueba.tex", "prueba_tikz.tex", "prueba.pdf", "prueba.png", "prueba.svg"]);
    for f in &report.files {
        assert!(std::fs::metadata(f).unwrap().len() > 100, "{} vacío", f.display());
    }
    let png = std::fs::read(dir.join("prueba.png")).unwrap();
    assert_eq!(&png[1..4], b"PNG");
    let snippet = std::fs::read_to_string(dir.join("prueba_tikz.tex")).unwrap();
    assert!(snippet.contains("%   \\usepackage{pgfplots}") && !snippet.contains("\\documentclass"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn compiles_with_pdflatex() {
    if !has_latex(LatexEngine::PdfLatex) {
        return;
    }
    let (mut p, scene) = example("imanes");
    p.figure.engine = LatexEngine::PdfLatex;
    let dir = temp_dir("pdflatex");
    let pdf = compile_pdf(&generate(&p, &scene).full, LatexEngine::PdfLatex, &dir, "g", COMPILE_TIMEOUT).unwrap();
    assert!(pdf.is_file());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn latex_errors_point_to_the_line() {
    if !has_latex(LatexEngine::LuaLatex) {
        return;
    }
    let (mut p, scene) = example("imanes");
    p.y_axis.label = "Periodo $\\comandoinexistente$".into();
    let dir = temp_dir("error");
    let err = compile_pdf(&generate(&p, &scene).full, LatexEngine::LuaLatex, &dir, "g", COMPILE_TIMEOUT).unwrap_err();
    match &err {
        RenderError::Latex { line, message, .. } => {
            assert!(line.is_some());
            assert!(message.contains("Undefined control sequence"), "{message}");
        }
        other => panic!("{other}"),
    }
    assert!(err.to_string().contains("comandoinexistente"), "{err}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Todas las fuentes con ambos motores (lento: `cargo test -- --ignored`).
#[test]
#[ignore]
fn all_fonts_compile() {
    let (mut p, scene) = example("imanes");
    let fonts = [
        FontChoice::LatinModern,
        FontChoice::Times,
        FontChoice::Palatino,
        FontChoice::Libertine,
        FontChoice::Charter,
        FontChoice::FiraSans,
        FontChoice::System("Cambria".into()),
    ];
    let mut failures = Vec::new();
    for engine in [LatexEngine::LuaLatex, LatexEngine::PdfLatex] {
        for font in &fonts {
            p.figure.engine = engine;
            p.figure.font = font.clone();
            let dir = temp_dir("fonts");
            let r = compile_pdf(&generate(&p, &scene).full, engine, &dir, "g", COMPILE_TIMEOUT);
            println!("{engine:?} {font:?}: {}", if r.is_ok() { "ok".into() } else { format!("{:?}", r.as_ref().err()) });
            if r.is_err() {
                failures.push(format!("{engine:?} {font:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}
