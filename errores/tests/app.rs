//! Pruebas de FisErr sin ventana (egui_kittest).

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fiserr::{ErrorFile, FisErrApp, examples};
use fisgraf::config::PropagationMethod;
use fisgraf::propagation::propagate;

fn harness(app: FisErrApp) -> Harness<'static, FisErrApp> {
    Harness::builder().with_size(egui::vec2(1300.0, 900.0)).build_ui_state(|ui, app: &mut FisErrApp| app.draw(ui), app)
}

#[test]
fn starts_with_pendulum_and_shows_result() {
    let mut h = harness(FisErrApp::default());
    h.run_steps(4);
    assert_eq!(h.state().file.calculations.len(), 1);
    assert!(!h.state().is_dirty());
    h.get_by_label_contains("g = 9,80 ± 0,05 m/s");
    h.get_by_label("Función final con su error");
    h.get_by_label_contains("σ_g = ");
}

#[test]
fn every_example_computes() {
    for (name, make) in examples::ALL {
        let c = make();
        let r = propagate(&c).unwrap_or_else(|e| panic!("{name}: {e}"));
        let lin = r.get(PropagationMethod::Linear);
        assert!(lin.sigma > 0.0 && lin.sigma.is_finite(), "{name}");
        assert!(r.warnings.iter().all(|w| !w.contains("no finito")), "{name}: {:?}", r.warnings);
    }
    // Densidad: ρ ≈ 2,72 g/cm³ (aluminio).
    let r = propagate(&examples::densidad()).unwrap();
    assert!((r.nominal - 2.72).abs() < 0.01, "{}", r.nominal);
}

#[test]
fn new_button_and_example_menu_add_calculations() {
    let mut h = harness(FisErrApp::default());
    h.run_steps(3);
    h.get_by_label("+ Nuevo").click();
    h.run_steps(3);
    assert_eq!(h.state().file.calculations.len(), 2);
    assert!(h.state().is_dirty());
    h.state_mut().add(examples::snell());
    h.run_steps(3);
    h.get_by_label_contains("n = 1,5");
}

#[test]
fn save_and_open_roundtrip() {
    let mut app = FisErrApp::default();
    app.add(examples::densidad());
    app.add(examples::paralelo());
    let path = std::env::temp_dir().join(format!("fiserr_{}.fiserr.json", std::process::id()));
    assert!(app.save_to(&path));
    let loaded = ErrorFile::load(&path).unwrap();
    assert_eq!(loaded, app.file);
    let mut other = FisErrApp::default();
    other.open_path(&path);
    assert_eq!(other.file.calculations.len(), 2);
    assert!(!other.is_dirty());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn about_window_shows_author_and_license() {
    let mut h = harness(FisErrApp::default());
    h.run_steps(3);
    h.get_by_label("Ayuda").click();
    h.run_steps(3);
    h.get_by_label("Acerca de FisErr…").click();
    h.run_steps(3);
    assert!(h.state().show_about);
    h.get_by_label_contains("Martín Álvarez de Araya");
    h.get_by_label_contains("citando siempre al autor");
    h.get_by_label("Cerrar").click();
    h.run_steps(3);
    assert!(!h.state().show_about);
}
