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
