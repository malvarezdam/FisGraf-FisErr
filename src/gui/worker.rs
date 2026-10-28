//! Cálculo en segundo plano: ajustes e incertidumbres pueden tardar (bootstrap,
//! perfil de χ²), así que se hacen en otro hilo y la interfaz sigue respondiendo.
//!
//! Los cambios se agrupan: se espera `DEBOUNCE` desde el último cambio y nunca
//! corre más de un cálculo a la vez; el resultado de un proyecto ya
//! modificado se descarta.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use crate::config::ProjectConfig;
use crate::data::Workbook;
use crate::pipeline::{Computed, build_scene, compute};
use crate::render::Scene;

pub const DEBOUNCE: Duration = Duration::from_millis(250);

pub struct Output {
    /// Proyecto con que se calculó.
    pub project: ProjectConfig,
    pub computed: Computed,
    pub scene: Scene,
    pub elapsed: Duration,
}

pub struct Worker {
    tx: Sender<Output>,
    rx: Receiver<Output>,
    running: bool,
    /// Proyecto que se está calculando ahora.
    in_flight: Option<ProjectConfig>,
    /// Proyecto que espera ser calculado y desde cuándo.
    pending: Option<(ProjectConfig, Instant)>,
    pub latest: Option<Arc<Output>>,
}

impl Default for Worker {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self { tx, rx, running: false, in_flight: None, pending: None, latest: None }
    }
}

impl Worker {
    /// Pide recalcular si `project` difiere del último estado conocido
    /// (pendiente, en curso o ya calculado, en ese orden).
    pub fn request(&mut self, project: &ProjectConfig) {
        let known = self
            .pending
            .as_ref()
            .map(|(p, _)| p)
            .or(self.in_flight.as_ref())
            .or(self.latest.as_ref().map(|o| &o.project));
        if known != Some(project) {
            self.pending = Some((project.clone(), Instant::now()));
        }
    }

    /// Fuerza un cálculo nuevo (p. ej. tras recargar el Excel).
    pub fn invalidate(&mut self, project: &ProjectConfig) {
        self.latest = None;
        self.pending = Some((project.clone(), Instant::now() - DEBOUNCE));
    }

    pub fn is_busy(&self) -> bool {
        self.running || self.pending.is_some()
    }

    /// Recoge resultados y lanza el siguiente cálculo si toca. Devuelve cuánto
    /// esperar para volver a llamar (`None` si no hay nada pendiente).
    pub fn poll(&mut self, book: Option<Arc<Workbook>>, wake: impl Fn() + Send + 'static) -> Option<Duration> {
        while let Ok(out) = self.rx.try_recv() {
            self.running = false;
            self.in_flight = None;
            self.latest = Some(Arc::new(out));
        }
        let (_, since) = self.pending.as_ref()?;
        if self.running {
            return Some(Duration::from_millis(50));
        }
        let wait = DEBOUNCE.saturating_sub(since.elapsed());
        if !wait.is_zero() {
            return Some(wait);
        }
        let (project, _) = self.pending.take().expect("pendiente");
        self.running = true;
        self.in_flight = Some(project.clone());
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let start = Instant::now();
            let computed = compute(&project, book.as_deref());
            let scene = build_scene(&project, &computed);
            let _ = tx.send(Output { project, computed, scene, elapsed: start.elapsed() });
            wake();
        });
        Some(Duration::from_millis(50))
    }

    /// Espera (bloqueando) a que termine todo; útil en pruebas.
    pub fn wait(&mut self, book: Option<Arc<Workbook>>) {
        let start = Instant::now();
        while self.is_busy() && start.elapsed() < Duration::from_secs(30) {
            if let Some(d) = self.poll(book.clone(), || {}) {
                std::thread::sleep(d.min(Duration::from_millis(20)));
            }
        }
    }
}
