//! Pruebas de la interfaz sin ventana (egui_kittest).

use std::path::PathBuf;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fisgraf::config::{ErrorSpec, FitConfig};
use fisgraf::data::CellRef;
use fisgraf::gui::{FisGrafApp, Tab};

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("examples/proyectos/{name}.fisgraf.json"))
}

fn harness(app: FisGrafApp) -> Harness<'static, FisGrafApp> {
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_ui_state(|ui, app: &mut FisGrafApp| app.draw(ui), app)
}

#[test]
fn new_series_button() {
    let mut app = FisGrafApp::default();
    app.tab = Tab::Series;
    let mut h = harness(app);
    h.run_steps(3);
    h.get_by_label("+ Nueva serie").click();
    h.run_steps(3);
    assert_eq!(h.state().project.series.len(), 1);
    assert!(h.state().is_dirty());
    h.get_by_label("Duplicar").click();
    h.run_steps(3);
    assert_eq!(h.state().project.series.len(), 2);
    assert_ne!(h.state().project.series[0].color, h.state().project.series[1].color);
}

#[test]
fn open_example_computes_and_draws_every_tab() {
    let mut app = FisGrafApp::default();
    app.open_path(&example("pendulo"));
    assert!(app.workbook.is_some());
    assert!(!app.is_dirty());
    let book = app.workbook.clone();
    app.worker.request(&app.project);
    app.worker.wait(book);
    let out = app.worker.latest.as_ref().expect("cálculo terminado");
    assert_eq!(out.scene.points.len(), 1);
    assert_eq!(out.scene.curves.len(), 1);
    assert!(out.scene.warnings.is_empty(), "{:?}", out.scene.warnings);

    let mut h = harness(app);
    for tab in [Tab::Data, Tab::Series, Tab::Fits] {
        h.state_mut().tab = tab;
        h.run_steps(3);
    }
    h.get_by_label("Vista rápida");
    h.state_mut().tab = Tab::Series;
    h.run_steps(3);
    h.get_by_label("✔ 9 puntos");
}

#[test]
fn selecting_cells_assigns_ranges() {
    let mut app = FisGrafApp::default();
    app.open_path(&example("imanes"));
    app.sheet = Some("Imanes".into());

    app.selection.anchor = Some(CellRef::new(1, 0));
    app.selection.cursor = Some(CellRef::new(4, 0));
    assert_eq!(app.selection_range().unwrap().to_string(), "Imanes!A2:A5");
    app.open_ended = true;
    assert_eq!(app.selection_range().unwrap().to_string(), "Imanes!A2:A");
    app.open_ended = false;

    // Tres columnas: x, y, σy; el nombre sale del encabezado de y.
    app.selection.cursor = Some(CellRef::new(4, 2));
    assert!(app.create_series_from_selection());
    let s = app.project.series.last().unwrap();
    assert_eq!(s.label, "T Tracker [s]");
    assert_eq!(s.x_range, "Imanes!A2:A5");
    assert_eq!(s.y_range, "Imanes!B2:B5");
    assert_eq!(s.y_error, ErrorSpec::Range("Imanes!C2:C5".into()));

    // Asignar una columna suelta a σx de la serie elegida.
    app.selection.anchor = Some(CellRef::new(1, 3));
    app.selection.cursor = Some(CellRef::new(4, 3));
    app.assign_selection(fisgraf::gui::Target::XError);
    assert_eq!(app.project.series.last().unwrap().x_error, ErrorSpec::Range("Imanes!D2:D5".into()));

    // Una selección de dos columnas no se puede asignar a x.
    app.selection.cursor = Some(CellRef::new(4, 4));
    let before = app.project.clone();
    app.assign_selection(fisgraf::gui::Target::XError);
    assert_eq!(app.project, before);
    assert!(app.messages.last().unwrap().error);
}

#[test]
fn undo_and_redo_through_frames() {
    let mut h = harness(FisGrafApp::default());
    h.run_steps(3);
    h.state_mut().project.figure.title = "Uno".into();
    h.run_steps(3);
    h.state_mut().project.figure.title = "Dos".into();
    h.run_steps(3);
    h.state_mut().undo();
    assert_eq!(h.state().project.figure.title, "Uno");
    h.state_mut().undo();
    assert_eq!(h.state().project.figure.title, "");
    h.state_mut().redo();
    assert_eq!(h.state().project.figure.title, "Uno");
}

#[test]
fn removing_series_updates_fits() {
    let mut app = FisGrafApp::default();
    let a = app.add_series();
    let b = app.add_series();
    app.project.fits.push(FitConfig { id: 99, series_ids: vec![a, b], ..Default::default() });
    app.remove_series(a);
    assert_eq!(app.project.fits[0].series_ids, [b]);
    assert_eq!(app.selected_series, Some(b));
    app.move_series(b, true);
    assert_eq!(app.project.series.len(), 1);
}

#[test]
fn save_marks_clean() {
    let mut app = FisGrafApp::default();
    app.add_series();
    assert!(app.is_dirty());
    let path = std::env::temp_dir().join(format!("fisgraf_gui_{}.fisgraf.json", std::process::id()));
    assert!(app.save_to(&path));
    assert!(!app.is_dirty());
    std::fs::remove_file(path).unwrap();
}

fn computed_app(name: &str) -> FisGrafApp {
    let mut app = FisGrafApp::default();
    app.open_path(&example(name));
    let book = app.workbook.clone();
    app.worker.request(&app.project);
    app.worker.wait(book);
    app
}

#[test]
fn all_tabs_and_results_draw() {
    let app = computed_app("pendulo");
    let mut h = harness(app);
    for tab in [Tab::Data, Tab::Series, Tab::Fits, Tab::References, Tab::Axes, Tab::Style, Tab::Export] {
        h.state_mut().tab = tab;
        h.run_steps(3);
    }
    // Panel de resultados: el parámetro g redondeado según la GUM.
    h.get_by_label("Copiar tabla LaTeX");
    h.get_by_label_contains("9,784 ± 0,022");
    h.state_mut().show_results = false;
    h.run_steps(2);
}

#[test]
fn new_fit_from_button_and_parameter_sync() {
    let mut app = FisGrafApp::default();
    app.open_path(&example("imanes"));
    app.tab = Tab::Fits;
    let mut h = harness(app);
    h.run_steps(3);
    h.get_by_label("+ Nuevo ajuste").click();
    h.run_steps(3);
    let app = h.state_mut();
    assert_eq!(app.project.fits.len(), 1);
    let series = app.project.series[0].id;
    assert_eq!(app.project.fits[0].series_ids, [series]);

    // Cambiar la ecuación y sincronizar parámetros conserva los que siguen.
    let f = &mut app.project.fits[0];
    f.model.parameters[0].initial = 0.0128;
    f.model.expression = "a*x + b + c*x^2".into();
    fisgraf::gui::sync_parameters(&mut f.model).unwrap();
    let names: Vec<_> = f.model.parameters.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["a", "b", "c"]);
    assert_eq!(f.model.parameters[0].initial, 0.0128);

    let book = app.workbook.clone();
    app.worker.request(&app.project);
    app.worker.wait(book);
    let out = app.worker.latest.as_ref().unwrap();
    let run = &out.computed.fits[0];
    let o = run.outcome.as_ref().unwrap();
    assert_eq!(o.result.method, fisgraf::config::FitMethod::EffectiveVariance);
    assert!(o.result.converged);
}

#[test]
fn results_copy_formats() {
    let app = computed_app("pendulo");
    let out = app.worker.latest.as_ref().unwrap();
    let o = out.computed.fits[0].outcome.as_ref().unwrap();
    let u = o.uncertainty.as_ref().unwrap();
    let cfg = &out.project.fits[0];
    let latex = fisgraf::gui::results::results_latex(cfg, o, u, true);
    assert!(latex.contains(r"$g$ & $9{,}784 \pm 0{,}022$\,\unit{m/s^2}"), "{latex}");
    assert!(latex.contains(r"$\chi^2_\nu$"));
    let text = fisgraf::gui::results::results_text(cfg, o, u, true);
    assert!(text.starts_with("g = 9,784 ± 0,022 m/s^2"), "{text}");
}

#[test]
fn references_tab_add_and_constants() {
    let mut app = FisGrafApp::default();
    app.tab = Tab::References;
    let mut h = harness(app);
    h.run_steps(3);
    h.get_by_label("+ Nueva referencia").click();
    h.run_steps(3);
    let r = &mut h.state_mut().project.references[0];
    r.expression = "A*sin(w*x)".into();
    h.run_steps(3);
    h.get_by_label("Agregarlas").click();
    h.run_steps(3);
    let names: Vec<_> = h.state().project.references[0].constants.iter().map(|c| c.name.clone()).collect();
    assert_eq!(names, ["A", "w"]);
}

#[test]
fn tick_list_parsing() {
    assert_eq!(fisgraf::gui::parse_tick_list("0,95; 1; 1,05"), Some(vec![0.95, 1.0, 1.05]));
}

#[test]
fn export_from_gui_writes_files() {
    if fisgraf::render::compile::find_program("lualatex").is_none() {
        return;
    }
    let mut app = FisGrafApp::default();
    app.open_path(&example("imanes"));
    let dir = std::env::temp_dir().join(format!("fisgraf_gui_export_{}", std::process::id()));
    app.project.export.output_dir = Some(dir.clone());
    app.project.export.png = false;
    assert_eq!(app.export_dir(), Some(dir.clone()));
    let report = fisgraf::gui::export_project(&app.project, app.workbook.as_deref(), &dir).unwrap();
    assert!(report.files.iter().any(|f| f.extension().is_some_and(|e| e == "pdf")));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn latex_preview_compiles_inside_the_app() {
    if fisgraf::render::compile::find_program("lualatex").is_none() {
        return;
    }
    let mut app = computed_app("imanes");
    app.latex_view = true;
    let mut h = harness(app);
    let start = std::time::Instant::now();
    while !h.state().latex_preview_ready() && start.elapsed() < std::time::Duration::from_secs(120) {
        h.run_steps(1);
        assert!(h.state().latex_preview_error().is_none(), "{:?}", h.state().latex_preview_error());
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(h.state().latex_preview_ready());
    h.get_by_label_contains("Compilado en");
}

#[test]
fn all_examples_compute_cleanly() {
    for name in ["imanes", "pendulo", "hooke", "decaimiento"] {
        let app = computed_app(name);
        let out = app.worker.latest.as_ref().unwrap();
        assert!(out.scene.warnings.is_empty(), "{name}: {:?}", out.scene.warnings);
        for run in &out.computed.fits {
            let o = run.outcome.as_ref().unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(o.result.converged, "{name}");
            assert!(o.uncertainty.is_ok(), "{name}");
        }
    }
}

#[test]
fn propagation_tab_end_to_end() {
    let mut app = FisGrafApp::default();
    app.tab = Tab::Propagation;
    let mut h = harness(app);
    h.run_steps(3);
    h.get_by_label("+ Nuevo cálculo").click();
    h.run_steps(3);
    {
        let p = &mut h.state_mut().project.propagations[0];
        p.name = "g".into();
        p.unit = "m/s^2".into();
        p.expression = "4*pi^2*L/T^2".into();
        p.samples = 20_000;
    }
    h.run_steps(3);
    h.get_by_label("Detectar variables").click();
    h.run_steps(3);
    {
        let p = &mut h.state_mut().project.propagations[0];
        assert_eq!(p.variables.len(), 2);
        p.variables[0].value = 1.0; // L
        p.variables[0].sigma = 0.001;
        p.variables[1].value = 2.007; // T
        p.variables[1].sigma = 0.005;
    }
    h.run_steps(3);
    h.get_by_label_contains("g = 9,80 ± 0,05 m/s²");
    h.get_by_label("Copiar fórmula de propagación");
    h.get_by_label_contains("Presupuesto de incertidumbre");
}

#[test]
fn test_data_projects_are_bundled_and_compute() {
    let found = fisgraf::gui::bundled_projects(&["datos_de_prueba", "proyectos"]);
    assert_eq!(found.len(), 8, "{found:?}");
    assert_eq!(found[0].0, "01 caida libre.fisgraf.json".trim_end_matches(".fisgraf.json"));
    for (name, path) in found {
        let mut app = FisGrafApp::default();
        app.open_path(&path);
        let book = app.workbook.clone();
        app.worker.request(&app.project);
        app.worker.wait(book);
        let out = app.worker.latest.as_ref().unwrap();
        assert!(out.scene.warnings.is_empty(), "{name}: {:?}", out.scene.warnings);
        for run in &out.computed.fits {
            let o = run.outcome.as_ref().unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(o.result.converged, "{name}");
        }
        for p in &app.project.propagations {
            fisgraf::propagation::propagate(p).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }
}
