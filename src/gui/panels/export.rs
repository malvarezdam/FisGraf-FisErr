//! Pestaña Exportar: formatos, carpeta y exportación en segundo plano.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::channel;

use egui::Ui;

use crate::config::ProjectConfig;
use crate::data::Workbook;
use crate::gui::FisGrafApp;
use crate::gui::widgets::{ERROR_COLOR, OK_COLOR};
use crate::pipeline::{build_scene, compute};
use crate::render::{ExportReport, export, generate};

/// Calcula y exporta el proyecto completo (se usa desde otro hilo).
pub fn export_project(project: &ProjectConfig, book: Option<&Workbook>, dir: &Path) -> Result<ExportReport, String> {
    let computed = compute(project, book);
    let scene = build_scene(project, &computed);
    let mut report = export(project, &scene, dir).map_err(|e| e.to_string())?;
    report.warnings.extend(scene.warnings);
    Ok(report)
}

/// Abre un archivo o carpeta con el programa predeterminado del sistema.
pub fn open_in_system(path: &Path) {
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(program).arg(path).spawn();
}

impl FisGrafApp {
    /// Carpeta de salida: la elegida, o la del proyecto guardado.
    pub fn export_dir(&self) -> Option<PathBuf> {
        self.project
            .export
            .output_dir
            .clone()
            .or_else(|| self.path.as_ref().and_then(|p| p.parent()).map(Path::to_path_buf))
    }

    pub fn start_export(&mut self, ctx: &egui::Context) {
        let Some(dir) = self.export_dir() else {
            self.error("Guarda el proyecto o elige una carpeta de salida antes de exportar.");
            return;
        };
        let (tx, rx) = channel();
        let project = self.project.clone();
        let book: Option<Arc<Workbook>> = self.workbook.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(export_project(&project, book.as_deref(), &dir));
            ctx.request_repaint();
        });
        self.export_job = Some(rx);
        self.info("Exportando…");
    }

    /// Recoge el resultado de la exportación en curso.
    pub(crate) fn poll_export(&mut self) {
        let Some(rx) = &self.export_job else { return };
        let Ok(result) = rx.try_recv() else { return };
        self.export_job = None;
        match result {
            Ok(report) => {
                let names: Vec<String> =
                    report.files.iter().filter_map(|f| f.file_name()).map(|n| n.to_string_lossy().into_owned()).collect();
                self.info(format!("Exportado: {}", names.join(", ")));
                for w in &report.warnings {
                    self.messages.push(crate::gui::Message { error: false, text: format!("⚠ {w}") });
                }
                self.last_export = Some(Ok(report));
            }
            Err(e) => {
                self.error(format!("Error al exportar: {e}"));
                self.last_export = Some(Err(e));
            }
        }
    }

    pub(crate) fn export_panel(&mut self, ui: &mut Ui) {
        ui.heading("Exportar");
        ui.strong("Carpeta de salida");
        ui.horizontal_wrapped(|ui| {
            match self.export_dir() {
                Some(d) => ui.monospace(d.display().to_string()),
                None => ui.colored_label(ERROR_COLOR, "Sin carpeta: guarda el proyecto o elige una"),
            };
        });
        ui.horizontal(|ui| {
            if ui.button("Elegir carpeta…").clicked()
                && let Some(d) = rfd::FileDialog::new().pick_folder()
            {
                self.project.export.output_dir = Some(d);
            }
            if self.project.export.output_dir.is_some() && ui.button("Junto al proyecto").clicked() {
                self.project.export.output_dir = None;
            }
            if let Some(d) = self.export_dir()
                && ui.button("Abrir carpeta").clicked()
            {
                open_in_system(&d);
            }
        });

        let e = &mut self.project.export;
        egui::Grid::new("exportar").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Nombre del archivo");
            ui.add(egui::TextEdit::singleline(&mut e.file_stem).desired_width(180.0));
            ui.end_row();
            ui.label("Formatos");
            ui.vertical(|ui| {
                ui.checkbox(&mut e.pdf, "PDF (vectorial, para LaTeX o imprimir)");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut e.png, "PNG");
                    if e.png {
                        ui.add(egui::DragValue::new(&mut e.png_dpi).range(72..=1200).suffix(" ppp"));
                    }
                });
                ui.checkbox(&mut e.svg, "SVG (vectorial, para web o Word)");
                ui.checkbox(&mut e.tex_standalone, ".tex completo (compilable)");
                ui.checkbox(&mut e.tex_snippet, "Fragmento tikzpicture (para \\input en tu informe)");
            });
            ui.end_row();
        });
        let nothing = !(e.pdf || e.png || e.svg || e.tex_standalone || e.tex_snippet);

        ui.add_space(6.0);
        let busy = self.export_job.is_some();
        ui.horizontal(|ui| {
            let button = egui::Button::new(egui::RichText::new("Exportar").strong()).min_size(egui::vec2(110.0, 28.0));
            if ui.add_enabled(!busy && !nothing, button).clicked() {
                self.start_export(ui.ctx());
            }
            if busy {
                ui.spinner();
                ui.label("Compilando con LaTeX…");
            }
        });

        // Copiar el código sin escribir archivos.
        ui.horizontal(|ui| {
            let latest = self.worker.latest.as_ref().filter(|o| o.project == self.project);
            if ui
                .add_enabled(latest.is_some(), egui::Button::new("Copiar código tikzpicture"))
                .on_hover_text("Para pegarlo directamente en tu documento LaTeX")
                .clicked()
                && let Some(o) = latest
            {
                let doc = generate(&self.project, &o.scene);
                ui.ctx().copy_text(doc.picture);
                self.info("Código tikzpicture copiado al portapapeles");
            }
        });

        match &self.last_export {
            Some(Ok(report)) => {
                ui.add_space(6.0);
                ui.colored_label(OK_COLOR, "✔ Archivos generados:");
                for f in &report.files {
                    ui.horizontal(|ui| {
                        ui.monospace(f.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned()));
                        if ui.small_button("Abrir").clicked() {
                            open_in_system(f);
                        }
                    });
                }
            }
            Some(Err(e)) => {
                ui.add_space(6.0);
                ui.colored_label(ERROR_COLOR, e);
            }
            None => {}
        }
    }
}
