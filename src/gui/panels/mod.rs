//! Pestañas de configuración del panel izquierdo.

mod axes;
mod data;
mod export;
mod fits;
mod propagation;
mod references;
mod series;
mod style;

pub use axes::parse_tick_list;
pub use data::Target;
pub use export::export_project;
pub use fits::sync_parameters;
pub use propagation::{result_latex, sync_variables};
