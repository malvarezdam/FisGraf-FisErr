//! Lectura de datos desde Excel (calamine) y rangos tipo `Hoja1!B2:B20`.

pub mod excel;
pub mod range;
pub mod series;

use std::path::PathBuf;

pub use excel::{Workbook, parse_number};
pub use range::{CellRange, CellRef, Orientation, RangeEnd, RangeError};
pub use series::{SeriesData, load_series};

#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("no se pudo abrir {}: {reason}", path.display())]
    Open { path: PathBuf, reason: String },

    #[error("no existe la hoja «{name}» (hojas disponibles: {available})")]
    SheetNotFound { name: String, available: String },

    #[error("{what}: {source}")]
    BadRange { what: String, source: RangeError },

    #[error(transparent)]
    Range(#[from] RangeError),

    #[error("la celda {cell} contiene «{value}», que no es un número")]
    NotNumeric { cell: String, value: String },

    #[error("{what} tienen distinto número de celdas ({a} y {b})")]
    LengthMismatch { what: String, a: usize, b: usize },

    #[error("falta el error de {axis} en la celda {cell}")]
    MissingError { axis: &'static str, cell: String },

    #[error("error de {axis} inválido ({detail}): debe ser un número ≥ 0")]
    InvalidError { axis: &'static str, detail: String },

    #[error("no hay ninguna fila con x e y a la vez")]
    NoData,

    #[error("serie «{series}»: {source}")]
    InSeries {
        series: String,
        #[source]
        source: Box<DataError>,
    },
}

impl DataError {
    /// Añade el nombre de la serie al mensaje.
    pub fn in_series(self, series: &str) -> Self {
        Self::InSeries {
            series: series.to_owned(),
            source: Box::new(self),
        }
    }
}
