//! Interfaz gráfica (egui).
//!
//! Distribución: menú arriba, pestañas de configuración a la izquierda, vista
//! previa en el centro y mensajes abajo. Los cálculos corren en segundo plano
//! ([`worker`]) y cada cambio queda en el historial para deshacer ([`history`]).

mod fonts;
pub mod history;
mod names;
mod panels;
mod preview;
pub mod results;
pub mod text;
pub mod widgets;
pub mod worker;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::{Key, KeyboardShortcut, Modifiers, RichText, Ui};

use crate::config::{PROJECT_EXTENSION, ProjectConfig};
use crate::data::{CellRef, Workbook};
use history::History;
pub use panels::{Target, export_project, parse_tick_list, sync_parameters};
use worker::Worker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Data,
    Series,
    Fits,
    References,
    Axes,
    Style,
    Export,
}

const TABS: [(Tab, &str); 7] = [
    (Tab::Data, "Datos"),
    (Tab::Series, "Series"),
    (Tab::Fits, "Ajustes"),
    (Tab::References, "Referencias"),
    (Tab::Axes, "Ejes"),
    (Tab::Style, "Estilo"),
    (Tab::Export, "Exportar"),
];

/// Acción que espera confirmación porque hay cambios sin guardar.
#[derive(Debug, Clone, PartialEq)]
enum Pending {
    New,
    Open(Option<PathBuf>),
    Close,
}

pub struct Message {
    pub error: bool,
    pub text: String,
}

/// Selección de celdas en la tabla del Excel: (ancla, cursor).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CellSelection {
    pub anchor: Option<CellRef>,
    pub cursor: Option<CellRef>,
}

pub struct FisGrafApp {
    pub project: ProjectConfig,
    pub path: Option<PathBuf>,
    /// Estado guardado en disco, para saber si hay cambios.
    saved: ProjectConfig,
    history: History,
    pub workbook: Option<Arc<Workbook>>,
    workbook_error: Option<String>,
    loaded_excel: Option<PathBuf>,
    pub worker: Worker,
    pub tab: Tab,
    pub selected_series: Option<u32>,
    /// Hoja mostrada en la pestaña Datos.
    pub sheet: Option<String>,
    pub selection: CellSelection,
    /// Al asignar una columna, usar un rango abierto (`B2:B`).
    pub open_ended: bool,
    preview_cache: Option<(String, Vec<Vec<String>>)>,
    pub messages: Vec<Message>,
    pending: Option<Pending>,
    close_confirmed: bool,
    title: String,
    fonts_installed: bool,
    pub selected_fit: Option<u32>,
    pub selected_reference: Option<u32>,
    /// Exportación en curso.
    export_job: Option<std::sync::mpsc::Receiver<Result<crate::render::ExportReport, String>>>,
    pub last_export: Option<Result<crate::render::ExportReport, String>>,
    /// Panel inferior: `true` = resultados, `false` = mensajes.
    pub show_results: bool,
}

impl Default for FisGrafApp {
    fn default() -> Self {
        let project = ProjectConfig::default();
        Self {
            history: History::new(&project),
            saved: project.clone(),
            project,
            path: None,
            workbook: None,
            workbook_error: None,
            loaded_excel: None,
            worker: Worker::default(),
            tab: Tab::Data,
            selected_series: None,
            sheet: None,
            selection: CellSelection::default(),
            open_ended: false,
            preview_cache: None,
            messages: Vec::new(),
            pending: None,
            close_confirmed: false,
            title: String::new(),
            fonts_installed: false,
            selected_fit: None,
            selected_reference: None,
            export_job: None,
            last_export: None,
            show_results: true,
        }
    }
}

impl FisGrafApp {
    pub fn is_dirty(&self) -> bool {
        self.project != self.saved
    }

    pub fn info(&mut self, text: impl Into<String>) {
        self.messages.push(Message { error: false, text: text.into() });
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.messages.push(Message { error: true, text: text.into() });
    }

    /// Reemplaza el proyecto actual (nuevo o abierto).
    fn replace_project(&mut self, project: ProjectConfig, path: Option<PathBuf>) {
        self.history = History::new(&project);
        self.saved = project.clone();
        self.selected_series = project.series.first().map(|s| s.id);
        self.project = project;
        self.path = path;
        self.selection = CellSelection::default();
        // La pestaña Datos muestra la hoja de la primera serie.
        self.sheet = self
            .project
            .series
            .first()
            .and_then(|s| s.x_range.parse::<crate::data::CellRange>().ok())
            .and_then(|r| r.sheet);
        self.loaded_excel = None;
        self.sync_workbook();
        self.worker.invalidate(&self.project);
    }

    pub fn new_project(&mut self) {
        self.replace_project(ProjectConfig::default(), None);
        self.info("Proyecto nuevo");
    }

    pub fn open_path(&mut self, path: &Path) {
        match ProjectConfig::load(path) {
            Ok(p) => {
                self.replace_project(p, Some(path.to_owned()));
                self.info(format!("Abierto {}", path.display()));
            }
            Err(e) => self.error(format!("{e:#}")),
        }
    }

    /// Guarda en la ruta actual o pregunta dónde. Devuelve si se guardó.
    pub fn save(&mut self) -> bool {
        match self.path.clone() {
            Some(p) => self.save_to(&p),
            None => self.save_as(),
        }
    }

    pub fn save_as(&mut self) -> bool {
        let mut dialog = rfd::FileDialog::new().add_filter("Proyecto FisGraf", &["json"]).set_file_name(format!("grafico.{PROJECT_EXTENSION}"));
        if let Some(dir) = self.path.as_ref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(dir);
        }
        match dialog.save_file() {
            Some(mut p) => {
                if !p.to_string_lossy().ends_with(PROJECT_EXTENSION) {
                    p = PathBuf::from(format!("{}.{PROJECT_EXTENSION}", p.with_extension("").display()));
                }
                self.save_to(&p)
            }
            None => false,
        }
    }

    pub fn save_to(&mut self, path: &Path) -> bool {
        match self.project.save(path) {
            Ok(()) => {
                self.saved = self.project.clone();
                self.path = Some(path.to_owned());
                self.info(format!("Guardado {}", path.display()));
                true
            }
            Err(e) => {
                self.error(format!("{e:#}"));
                false
            }
        }
    }

    pub fn undo(&mut self) {
        if let Some(p) = self.history.undo() {
            self.project = p;
        }
    }

    pub fn redo(&mut self) {
        if let Some(p) = self.history.redo() {
            self.project = p;
        }
    }

    /// Carga el Excel del proyecto si cambió la ruta.
    pub fn sync_workbook(&mut self) {
        if self.project.excel_path == self.loaded_excel {
            return;
        }
        self.loaded_excel = self.project.excel_path.clone();
        self.preview_cache = None;
        self.workbook = None;
        self.workbook_error = None;
        if let Some(path) = &self.loaded_excel {
            match Workbook::open(path) {
                Ok(b) => {
                    if self.sheet.as_ref().is_none_or(|s| !b.sheet_names().any(|n| n == s)) {
                        self.sheet = b.sheet_names().next().map(str::to_owned);
                    }
                    self.workbook = Some(Arc::new(b));
                }
                Err(e) => self.workbook_error = Some(e.to_string()),
            }
        }
        self.worker.invalidate(&self.project);
    }

    /// Vuelve a leer el Excel desde el disco (si se editó por fuera).
    pub fn reload_workbook(&mut self) {
        self.loaded_excel = None;
        self.sync_workbook();
        if self.workbook.is_some() {
            self.info("Excel recargado");
        }
    }

    /// Ejecuta una acción, o pide confirmación si hay cambios sin guardar.
    fn request(&mut self, action: Pending) {
        if self.is_dirty() {
            self.pending = Some(action);
        } else {
            self.perform(action);
        }
    }

    fn perform(&mut self, action: Pending) {
        match action {
            Pending::New => self.new_project(),
            Pending::Open(Some(p)) => self.open_path(&p),
            Pending::Open(None) => {
                if let Some(p) = rfd::FileDialog::new().add_filter("Proyecto FisGraf", &["json"]).pick_file() {
                    self.open_path(&p);
                }
            }
            Pending::Close => self.close_confirmed = true,
        }
    }

    fn shortcuts(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let typing = ctx.memory(|m| m.focused().is_some());
        let sc = |m: Modifiers, k: Key| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));
        if sc(Modifiers::COMMAND | Modifiers::SHIFT, Key::S) {
            self.save_as();
        } else if sc(Modifiers::COMMAND, Key::S) {
            self.save();
        }
        if sc(Modifiers::COMMAND, Key::O) {
            self.request(Pending::Open(None));
        }
        if sc(Modifiers::COMMAND, Key::N) {
            self.request(Pending::New);
        }
        // Mientras se escribe, Ctrl+Z pertenece al campo de texto.
        if !typing {
            if sc(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z) || sc(Modifiers::COMMAND, Key::Y) {
                self.redo();
            } else if sc(Modifiers::COMMAND, Key::Z) {
                self.undo();
            }
        }
    }

    fn menu(&mut self, ui: &mut Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("Archivo", |ui| {
                if ui.button("Nuevo                Ctrl+N").clicked() {
                    self.request(Pending::New);
                    ui.close();
                }
                if ui.button("Abrir…               Ctrl+O").clicked() {
                    self.request(Pending::Open(None));
                    ui.close();
                }
                let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples").join("proyectos");
                if examples.is_dir() {
                    ui.menu_button("Abrir ejemplo", |ui| {
                        for name in ["imanes", "pendulo"] {
                            let p = examples.join(format!("{name}.{PROJECT_EXTENSION}"));
                            if p.is_file() && ui.button(name).clicked() {
                                self.request(Pending::Open(Some(p)));
                                ui.close();
                            }
                        }
                    });
                }
                ui.separator();
                if ui.button("Guardar              Ctrl+S").clicked() {
                    self.save();
                    ui.close();
                }
                if ui.button("Guardar como…  Ctrl+Mayús+S").clicked() {
                    self.save_as();
                    ui.close();
                }
                ui.separator();
                if ui.button("Salir").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    ui.close();
                }
            });
            ui.menu_button("Editar", |ui| {
                if ui.add_enabled(self.history.can_undo(), egui::Button::new("Deshacer   Ctrl+Z")).clicked() {
                    self.undo();
                    ui.close();
                }
                if ui.add_enabled(self.history.can_redo(), egui::Button::new("Rehacer     Ctrl+Y")).clicked() {
                    self.redo();
                    ui.close();
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.worker.is_busy() {
                    ui.spinner();
                    ui.label("Calculando…");
                } else if let Some(o) = &self.worker.latest {
                    ui.weak(format!("Calculado en {} ms", o.elapsed.as_millis()));
                }
            });
        });
    }

    fn tabs(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            for (tab, name) in TABS {
                ui.selectable_value(&mut self.tab, tab, RichText::new(name).strong());
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| match self.tab {
            Tab::Data => self.data_panel(ui),
            Tab::Series => self.series_panel(ui),
            Tab::Fits => self.fits_panel(ui),
            Tab::References => self.references_panel(ui),
            Tab::Axes => self.axes_panel(ui),
            Tab::Style => self.style_panel(ui),
            Tab::Export => self.export_panel(ui),
        });
    }

    fn status_panel(&mut self, ui: &mut Ui) {
        let warnings = self.worker.latest.as_ref().map_or(0, |o| o.scene.warnings.len());
        let errors = self.messages.iter().filter(|m| m.error).count() + usize::from(self.workbook_error.is_some());
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.show_results, true, RichText::new("Resultados").strong());
            let label = match (warnings, errors) {
                (0, 0) => "Mensajes".to_owned(),
                (w, 0) => format!("Mensajes (⚠ {w})"),
                (w, e) => format!("Mensajes (⚠ {w}, ✖ {e})"),
            };
            ui.selectable_value(&mut self.show_results, false, RichText::new(label).strong());
        });
        ui.separator();
        if self.show_results {
            self.results_panel(ui);
            return;
        }
        egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink(false).show(ui, |ui| {
            if let Some(e) = &self.workbook_error {
                ui.colored_label(widgets::ERROR_COLOR, format!("Excel: {e}"));
            }
            if let Some(o) = &self.worker.latest {
                for w in &o.scene.warnings {
                    ui.colored_label(egui::Color32::from_rgb(0xC0, 0x80, 0x00), format!("⚠ {w}"));
                }
            }
            let start = self.messages.len().saturating_sub(30);
            for m in &self.messages[start..] {
                if m.error {
                    ui.colored_label(widgets::ERROR_COLOR, &m.text);
                } else {
                    ui.weak(&m.text);
                }
            }
        });
    }

    fn confirm_modal(&mut self, ui: &mut Ui) {
        let Some(action) = self.pending.clone() else { return };
        let mut choice = None;
        egui::Modal::new(egui::Id::unique("confirmar_cambios")).show(ui.ctx(), |ui| {
            ui.heading("Hay cambios sin guardar");
            ui.label("¿Quieres guardar el proyecto antes de continuar?");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Guardar").clicked() {
                    choice = Some(1);
                }
                if ui.button("No guardar").clicked() {
                    choice = Some(2);
                }
                if ui.button("Cancelar").clicked() {
                    choice = Some(3);
                }
            });
        });
        match choice {
            Some(1) => {
                self.pending = None;
                if self.save() {
                    self.perform(action);
                }
            }
            Some(2) => {
                self.pending = None;
                self.perform(action);
            }
            Some(3) => self.pending = None,
            _ => {}
        }
    }

    /// Dibuja toda la interfaz. Separado de `eframe::App` para poder probarlo.
    pub fn draw(&mut self, ui: &mut Ui) {
        if !self.fonts_installed {
            fonts::install(ui.ctx());
            self.fonts_installed = true;
        }
        self.sync_workbook();
        self.poll_export();
        self.shortcuts(ui);

        egui::Panel::top("menu").show(ui, |ui| self.menu(ui));
        egui::Panel::bottom("mensajes").resizable(true).default_size(220.0).min_size(60.0).show(ui, |ui| self.status_panel(ui));
        egui::Panel::left("controles").resizable(true).default_size(470.0).min_size(320.0).show(ui, |ui| self.tabs(ui));
        egui::CentralPanel::default_margins().show(ui, |ui| {
            let busy = self.worker.is_busy();
            preview::quick_preview(ui, &self.project, self.worker.latest.as_deref(), busy);
        });
        self.confirm_modal(ui);

        let ctx = ui.ctx().clone();
        // Cierre de la ventana con cambios sin guardar.
        if ctx.input(|i| i.viewport().close_requested()) && self.is_dirty() && !self.close_confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Close);
        }
        if self.close_confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Historial: se confirma cuando no se está arrastrando ni escribiendo.
        let idle = !ctx.input(|i| i.pointer.any_down()) && ctx.memory(|m| m.focused().is_none());
        if idle {
            self.history.commit(&self.project);
        }

        self.worker.request(&self.project);
        let wake_ctx = ctx.clone();
        if let Some(wait) = self.worker.poll(self.workbook.clone(), move || wake_ctx.request_repaint()) {
            ctx.request_repaint_after(wait);
        }

        let name = self.path.as_ref().and_then(|p| p.file_name()).map_or("Sin título".into(), |n| n.to_string_lossy().into_owned());
        let title = format!("{}{name} — FisGraf", if self.is_dirty() { "● " } else { "" });
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }
}

impl eframe::App for FisGrafApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}
