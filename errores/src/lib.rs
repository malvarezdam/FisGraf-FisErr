// FisGraf y FisErr © 2026 Martín Álvarez de Araya. Todos los derechos reservados (ver LICENSE).
//! FisErr: calculadora de propagación de errores.
//!
//! Usa la misma lógica que la pestaña Propagación de FisGraf
//! ([`fisgraf::propagation`] y [`fisgraf::gui::calculator`]): ecuación teórica,
//! variables medidas con su error, constantes, y el resultado con su error por
//! cinco métodos.

pub mod examples;

use std::path::{Path, PathBuf};

use egui::{Key, KeyboardShortcut, Modifiers, RichText, Ui};
use fisgraf::config::PropagationConfig;
use fisgraf::gui::calculator::Calculator;
use fisgraf::gui::widgets::ERROR_COLOR;
use serde::{Deserialize, Serialize};

pub const FILE_EXTENSION: &str = "fiserr.json";

/// Archivo de FisErr: una lista de cálculos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ErrorFile {
    pub format_version: u32,
    pub decimal_comma: bool,
    pub calculations: Vec<PropagationConfig>,
}

impl Default for ErrorFile {
    fn default() -> Self {
        Self { format_version: 1, decimal_comma: true, calculations: Vec::new() }
    }
}

impl ErrorFile {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text)?)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn next_id(&self) -> u32 {
        self.calculations.iter().map(|c| c.id).max().map_or(1, |m| m + 1)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Pending {
    New,
    Open,
    Close,
}

pub struct FisErrApp {
    pub file: ErrorFile,
    pub path: Option<PathBuf>,
    saved: ErrorFile,
    pub selected: Option<u32>,
    calculator: Calculator,
    pub message: Option<(bool, String)>,
    pending: Option<Pending>,
    close_confirmed: bool,
    title: String,
    started: bool,
    /// Ventana «Acerca de» abierta.
    pub show_about: bool,
}

impl Default for FisErrApp {
    fn default() -> Self {
        let file = ErrorFile::default();
        Self {
            saved: file.clone(),
            file,
            path: None,
            selected: None,
            calculator: Calculator::default(),
            message: None,
            pending: None,
            close_confirmed: false,
            title: String::new(),
            started: false,
            show_about: false,
        }
    }
}

impl FisErrApp {
    pub fn is_dirty(&self) -> bool {
        self.file != self.saved
    }

    /// Agrega un cálculo (vacío o un ejemplo) y lo selecciona.
    pub fn add(&mut self, mut calc: PropagationConfig) -> u32 {
        calc.id = self.file.next_id();
        let id = calc.id;
        self.file.calculations.push(calc);
        self.selected = Some(id);
        id
    }

    pub fn open_path(&mut self, path: &Path) {
        match ErrorFile::load(path) {
            Ok(f) => {
                self.saved = f.clone();
                self.selected = f.calculations.first().map(|c| c.id);
                self.file = f;
                self.path = Some(path.to_owned());
                self.message = Some((false, format!("Abierto {}", path.display())));
            }
            Err(e) => self.message = Some((true, format!("No se pudo abrir {}: {e:#}", path.display()))),
        }
    }

    pub fn save_to(&mut self, path: &Path) -> bool {
        match self.file.save(path) {
            Ok(()) => {
                self.saved = self.file.clone();
                self.path = Some(path.to_owned());
                self.message = Some((false, format!("Guardado {}", path.display())));
                true
            }
            Err(e) => {
                self.message = Some((true, format!("No se pudo guardar: {e:#}")));
                false
            }
        }
    }

    fn save(&mut self) -> bool {
        if let Some(p) = self.path.clone() {
            return self.save_to(&p);
        }
        let Some(mut p) = rfd::FileDialog::new().add_filter("Cálculos de FisErr", &["json"]).set_file_name(format!("errores.{FILE_EXTENSION}")).save_file()
        else {
            return false;
        };
        if !p.to_string_lossy().ends_with(FILE_EXTENSION) {
            p = PathBuf::from(format!("{}.{FILE_EXTENSION}", p.with_extension("").display()));
        }
        self.save_to(&p)
    }

    fn perform(&mut self, action: Pending) {
        match action {
            Pending::New => {
                *self = Self { started: true, ..Self::default() };
            }
            Pending::Open => {
                if let Some(p) = rfd::FileDialog::new().add_filter("Cálculos de FisErr", &["json"]).pick_file() {
                    self.open_path(&p);
                }
            }
            Pending::Close => self.close_confirmed = true,
        }
    }

    fn request(&mut self, action: Pending) {
        if self.is_dirty() { self.pending = Some(action) } else { self.perform(action) }
    }

    fn menu(&mut self, ui: &mut Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("Archivo", |ui| {
                if ui.button("Nuevo            Ctrl+N").clicked() {
                    self.request(Pending::New);
                    ui.close();
                }
                if ui.button("Abrir…           Ctrl+O").clicked() {
                    self.request(Pending::Open);
                    ui.close();
                }
                if ui.button("Guardar          Ctrl+S").clicked() {
                    self.save();
                    ui.close();
                }
                ui.separator();
                if ui.button("Salir").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    ui.close();
                }
            });
            ui.menu_button("Ejemplos", |ui| {
                for (name, make) in examples::ALL {
                    if ui.button(name).clicked() {
                        self.add(make());
                        ui.close();
                    }
                }
            });
            ui.menu_button("Ver", |ui| {
                ui.checkbox(&mut self.file.decimal_comma, "Coma decimal (9,80)");
            });
            ui.menu_button("Ayuda", |ui| {
                if ui.button("Acerca de FisErr…").clicked() {
                    self.show_about = true;
                    ui.close();
                }
            });
        });
    }

    fn list(&mut self, ui: &mut Ui) {
        ui.heading("Cálculos");
        ui.horizontal(|ui| {
            if ui.button("+ Nuevo").clicked() {
                self.add(PropagationConfig { name: "f".into(), ..Default::default() });
            }
            if let Some(id) = self.selected {
                if ui.button("Duplicar").clicked()
                    && let Some(c) = self.file.calculations.iter().find(|c| c.id == id).cloned()
                {
                    self.add(c);
                }
                if ui.button("Eliminar").clicked() {
                    self.file.calculations.retain(|c| c.id != id);
                    self.selected = self.file.calculations.first().map(|c| c.id);
                }
            }
        });
        ui.separator();
        if self.file.calculations.is_empty() {
            ui.label("Crea un cálculo con «+ Nuevo» o abre uno en el menú Ejemplos.");
        }
        for c in &self.file.calculations {
            let text = format!("{} = {}", c.name, if c.expression.is_empty() { "…" } else { &c.expression });
            if ui.selectable_label(self.selected == Some(c.id), text).clicked() {
                self.selected = Some(c.id);
            }
        }
    }

    fn confirm_modal(&mut self, ui: &mut Ui) {
        let Some(action) = self.pending else { return };
        let mut choice = 0;
        egui::Modal::new(egui::Id::unique("fiserr_confirmar")).show(ui.ctx(), |ui| {
            ui.heading("Hay cambios sin guardar");
            ui.horizontal(|ui| {
                if ui.button("Guardar").clicked() {
                    choice = 1;
                }
                if ui.button("No guardar").clicked() {
                    choice = 2;
                }
                if ui.button("Cancelar").clicked() {
                    choice = 3;
                }
            });
        });
        match choice {
            1 => {
                self.pending = None;
                if self.save() {
                    self.perform(action);
                }
            }
            2 => {
                self.pending = None;
                self.perform(action);
            }
            3 => self.pending = None,
            _ => {}
        }
    }

    pub fn draw(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        if !self.started {
            fisgraf::gui::fonts::install(&ctx);
            ctx.set_theme(egui::Theme::Dark);
            self.started = true;
            if self.file.calculations.is_empty() {
                self.add(examples::pendulo());
                self.saved = self.file.clone();
            }
        }
        let sc = |m: Modifiers, k: Key| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));
        if sc(Modifiers::COMMAND, Key::S) {
            self.save();
        }
        if sc(Modifiers::COMMAND, Key::O) {
            self.request(Pending::Open);
        }
        if sc(Modifiers::COMMAND, Key::N) {
            self.request(Pending::New);
        }

        egui::Panel::top("menu").show(ui, |ui| self.menu(ui));
        egui::Panel::bottom("estado").show(ui, |ui| match &self.message {
            Some((true, m)) => {
                ui.colored_label(ERROR_COLOR, m);
            }
            Some((false, m)) => {
                ui.weak(m);
            }
            None => {
                ui.weak("Ecuación con variables medidas (con su σ) y constantes → valor final con su error.");
            }
        });
        egui::Panel::left("lista").resizable(true).default_size(260.0).show(ui, |ui| self.list(ui));
        egui::CentralPanel::default_margins().show(ui, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                ui.label(RichText::new("Propagación de errores").heading());
                let comma = self.file.decimal_comma;
                let Some(id) = self.selected else { return };
                let Some(cfg) = self.file.calculations.iter_mut().find(|c| c.id == id) else { return };
                self.calculator.ui(ui, cfg, comma);
            });
        });
        self.confirm_modal(ui);
        fisgraf::gui::about::show(ui, &mut self.show_about, "FisErr", "Calculadora de propagación de errores.");

        if ctx.input(|i| i.viewport().close_requested()) && self.is_dirty() && !self.close_confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Close);
        }
        if self.close_confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let name = self.path.as_ref().and_then(|p| p.file_name()).map_or("Sin título".into(), |n| n.to_string_lossy().into_owned());
        let title = format!("{}{name} — FisErr", if self.is_dirty() { "● " } else { "" });
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }
}

impl eframe::App for FisErrApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}
