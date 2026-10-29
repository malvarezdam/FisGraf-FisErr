//! Deshacer / rehacer por instantáneas completas del proyecto.
//! Los proyectos pesan pocos kilobytes, así que copiar es más simple y seguro
//! que registrar cada operación.

use crate::config::ProjectConfig;

pub struct History {
    undo: Vec<ProjectConfig>,
    redo: Vec<ProjectConfig>,
    /// Último estado confirmado.
    current: ProjectConfig,
    limit: usize,
}

impl History {
    pub fn new(project: &ProjectConfig) -> Self {
        Self { undo: Vec::new(), redo: Vec::new(), current: project.clone(), limit: 200 }
    }

    /// Registra `project` si cambió respecto del último estado. Devuelve si hubo cambio.
    pub fn commit(&mut self, project: &ProjectConfig) -> bool {
        if *project == self.current {
            return false;
        }
        self.undo.push(std::mem::replace(&mut self.current, project.clone()));
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
        true
    }

    pub fn undo(&mut self) -> Option<ProjectConfig> {
        let prev = self.undo.pop()?;
        self.redo.push(std::mem::replace(&mut self.current, prev.clone()));
        Some(prev)
    }

    pub fn redo(&mut self) -> Option<ProjectConfig> {
        let next = self.redo.pop()?;
        self.undo.push(std::mem::replace(&mut self.current, next.clone()));
        Some(next)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo() {
        let mut p = ProjectConfig::default();
        let mut h = History::new(&p);
        assert!(!h.commit(&p));
        p.figure.title = "A".into();
        assert!(h.commit(&p));
        p.figure.title = "B".into();
        assert!(h.commit(&p));

        assert_eq!(h.undo().unwrap().figure.title, "A");
        assert_eq!(h.undo().unwrap().figure.title, "");
        assert!(h.undo().is_none());
        assert_eq!(h.redo().unwrap().figure.title, "A");

        // Un cambio nuevo descarta lo que se podía rehacer.
        let mut q = ProjectConfig::default();
        q.figure.title = "C".into();
        assert!(h.commit(&q));
        assert!(!h.can_redo());
        assert_eq!(h.undo().unwrap().figure.title, "A");
    }
}
