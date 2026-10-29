//! Vista previa con LaTeX real: compila el gráfico en segundo plano, lo
//! convierte a PNG y lo muestra como imagen.
//!
//! Se compila solo cuando el cálculo del proyecto actual terminó, nunca dos
//! veces a la vez, y en una carpeta propia para no pisar una exportación.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use egui::{ColorImage, TextureHandle, TextureOptions, Ui};

use crate::config::ProjectConfig;
use crate::gui::widgets::ERROR_COLOR;
use crate::gui::worker::Output;
use crate::render::compile::compile_pdf;
use crate::render::convert::pdf_to_png;
use crate::render::export::{COMPILE_TIMEOUT, build_dir};
use crate::render::generate;

/// Resolución del PNG de la vista previa.
const DPI: u32 = 200;

struct Rendered {
    project: ProjectConfig,
    image: ColorImage,
    elapsed: Duration,
}

#[derive(Default)]
pub struct LatexPreview {
    job: Option<(ProjectConfig, Receiver<Result<Rendered, String>>)>,
    /// Proyecto de la última imagen o error.
    shown_for: Option<ProjectConfig>,
    texture: Option<TextureHandle>,
    error: Option<String>,
    elapsed: Option<Duration>,
}

fn work_dir() -> PathBuf {
    build_dir().join("vista")
}

/// Compila y rasteriza (en otro hilo).
fn render(project: ProjectConfig, out: Arc<Output>) -> Result<Rendered, String> {
    let start = Instant::now();
    let doc = generate(&project, &out.scene);
    let dir = work_dir();
    let pdf = compile_pdf(&doc.full, project.figure.engine, &dir, "vista_previa", COMPILE_TIMEOUT).map_err(|e| e.to_string())?;
    let png = dir.join("vista_previa.png");
    pdf_to_png(&pdf, &png, DPI).map_err(|e| e.to_string())?;
    let bytes = std::fs::read(&png).map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let image = ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Ok(Rendered { project, image, elapsed: start.elapsed() })
}

impl LatexPreview {
    pub fn has_image(&self) -> bool {
        self.texture.is_some()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }

    /// Recoge resultados y lanza una compilación si hace falta.
    /// `latest` debe ser el cálculo del proyecto actual.
    pub fn update(&mut self, ctx: &egui::Context, project: &ProjectConfig, latest: Option<&Arc<Output>>) {
        if let Some((_, rx)) = &self.job
            && let Ok(result) = rx.try_recv()
        {
            let (for_project, _) = self.job.take().expect("hay trabajo");
            match result {
                Ok(r) => {
                    self.texture = Some(ctx.load_texture("vista_latex", r.image, TextureOptions::LINEAR));
                    self.elapsed = Some(r.elapsed);
                    self.error = None;
                    self.shown_for = Some(r.project);
                }
                Err(e) => {
                    self.error = Some(e);
                    self.shown_for = Some(for_project);
                }
            }
        }
        if self.job.is_some() || self.shown_for.as_ref() == Some(project) {
            return;
        }
        // Solo con el cálculo del proyecto actual ya terminado.
        let Some(out) = latest.filter(|o| o.project == *project) else { return };
        let (tx, rx) = channel();
        let (p, o, ctx2) = (project.clone(), out.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(render(p, o));
            ctx2.request_repaint();
        });
        self.job = Some((project.clone(), rx));
    }

    pub fn ui(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if self.job.is_some() {
                ui.spinner();
                ui.label("Compilando con LaTeX…");
            } else if let Some(t) = self.elapsed {
                ui.weak(format!("Compilado en {:.1} s", t.as_secs_f32()));
            }
        });
        if let Some(e) = &self.error {
            ui.colored_label(ERROR_COLOR, "No se pudo compilar el gráfico:");
            egui::ScrollArea::vertical().max_height(160.0).id_salt("error_latex").show(ui, |ui| {
                ui.monospace(e);
            });
            ui.weak("Revisa las etiquetas y leyendas: todo texto del gráfico se interpreta como LaTeX.");
        }
        let Some(tex) = &self.texture else {
            if self.error.is_none() && self.job.is_none() {
                ui.weak("La vista LaTeX aparece cuando hay algo que dibujar.");
            }
            return;
        };
        // Ajusta la imagen al espacio disponible manteniendo la proporción.
        let avail = ui.available_size();
        let size = tex.size_vec2();
        let scale = (avail.x / size.x).min(avail.y / size.y).min(1.5);
        egui::Frame::new().fill(egui::Color32::WHITE).inner_margin(6.0).show(ui, |ui| {
            ui.add(egui::Image::new((tex.id(), size * scale)));
        });
        if self.error.is_some() {
            ui.weak("(se muestra la última versión que compiló)");
        }
    }
}
