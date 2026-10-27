//! Construye los datos numéricos de una serie a partir de su configuración.

use crate::config::{ErrorSpec, SeriesConfig};

use super::DataError;
use super::excel::Workbook;
use super::range::CellRange;

/// Datos listos para ajustar y dibujar. Todos los vectores tienen la misma longitud.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SeriesData {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    /// Incertidumbre estándar de x (σ), si se configuró.
    pub x_err: Option<Vec<f64>>,
    /// Incertidumbre estándar de y (σ), si se configuró.
    pub y_err: Option<Vec<f64>>,
    /// Posiciones del rango (base 0) descartadas por tener x o y vacías.
    pub skipped: Vec<usize>,
}

impl SeriesData {
    pub fn len(&self) -> usize {
        self.x.len()
    }

    pub fn is_empty(&self) -> bool {
        self.x.is_empty()
    }
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

impl Axis {
    fn name(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
        }
    }
}

/// Lee una serie del libro: valores de x e y y sus incertidumbres.
///
/// Las filas en que x o y están vacías se descartan (quedan en `skipped`).
/// Si una fila válida no tiene error en la columna de errores, se informa.
pub fn load_series(book: &Workbook, cfg: &SeriesConfig) -> Result<SeriesData, DataError> {
    let ctx = |e: DataError| e.in_series(&cfg.label);

    let x_range = parse_range(&cfg.x_range, Axis::X).map_err(ctx)?;
    let y_range = parse_range(&cfg.y_range, Axis::Y).map_err(ctx)?;
    let x = book.read_vector(&x_range).map_err(ctx)?;
    let y = book.read_vector(&y_range).map_err(ctx)?;
    if x.len() != y.len() {
        return Err(ctx(DataError::LengthMismatch {
            what: "x e y".into(),
            a: x.len(),
            b: y.len(),
        }));
    }

    let x_err = resolve_errors(book, &cfg.x_error, &x, Axis::X).map_err(ctx)?;
    let y_err = resolve_errors(book, &cfg.y_error, &y, Axis::Y).map_err(ctx)?;

    let mut out = SeriesData {
        x_err: x_err.as_ref().map(|_| Vec::new()),
        y_err: y_err.as_ref().map(|_| Vec::new()),
        ..Default::default()
    };
    for i in 0..x.len() {
        let (Some(xi), Some(yi)) = (x[i], y[i]) else {
            out.skipped.push(i);
            continue;
        };
        out.x.push(xi);
        out.y.push(yi);
        for (errs, dest, axis) in [(&x_err, &mut out.x_err, Axis::X), (&y_err, &mut out.y_err, Axis::Y)] {
            if let (Some(errs), Some(dest)) = (errs, dest) {
                let e = errs[i].ok_or_else(|| {
                    let cell = match (axis, &cfg.x_error, &cfg.y_error) {
                        (Axis::X, ErrorSpec::Range(r), _) | (Axis::Y, _, ErrorSpec::Range(r)) => r
                            .parse::<CellRange>()
                            .map(|r| book.nth_cell_name(&r, i))
                            .unwrap_or_default(),
                        _ => String::new(),
                    };
                    ctx(DataError::MissingError { axis: axis.name(), cell })
                })?;
                dest.push(e);
            }
        }
    }

    if out.is_empty() {
        return Err(ctx(DataError::NoData));
    }
    Ok(out)
}

fn parse_range(text: &str, axis: Axis) -> Result<CellRange, DataError> {
    text.parse().map_err(|source| DataError::BadRange {
        what: format!("rango de {}", axis.name()),
        source,
    })
}

/// Errores por posición del rango (alineados con `values`), o `None` si no hay.
fn resolve_errors(
    book: &Workbook,
    spec: &ErrorSpec,
    values: &[Option<f64>],
    axis: Axis,
) -> Result<Option<Vec<Option<f64>>>, DataError> {
    let check = |v: f64, what: &str| {
        if v.is_finite() && v >= 0.0 {
            Ok(v)
        } else {
            Err(DataError::InvalidError {
                axis: axis.name(),
                detail: format!("{what} = {v}"),
            })
        }
    };

    let errs = match spec {
        ErrorSpec::None => return Ok(None),
        ErrorSpec::Constant(c) => vec![Some(check(*c, "error constante")?); values.len()],
        ErrorSpec::Percent(p) => {
            let p = check(*p, "porcentaje")?;
            values.iter().map(|v| v.map(|v| v.abs() * p / 100.0)).collect()
        }
        ErrorSpec::Resolution(r) => {
            let sigma = check(*r, "resolución")? / (2.0 * 3f64.sqrt());
            vec![Some(sigma); values.len()]
        }
        ErrorSpec::Range(text) => {
            let range: CellRange = text.parse().map_err(|source| DataError::BadRange {
                what: format!("rango de errores de {}", axis.name()),
                source,
            })?;
            let mut errs = book.read_vector(&range)?;
            // Un rango abierto pierde las celdas vacías del final; se rellenan.
            if errs.len() < values.len() && matches!(range.end, super::range::RangeEnd::OpenColumn(_)) {
                errs.resize(values.len(), None);
            }
            if errs.len() != values.len() {
                return Err(DataError::LengthMismatch {
                    what: format!("{} y su error", axis.name()),
                    a: values.len(),
                    b: errs.len(),
                });
            }
            for (i, e) in errs.iter().enumerate() {
                if let Some(e) = e {
                    check(*e, &book.nth_cell_name(&range, i))?;
                }
            }
            errs
        }
    };
    Ok(Some(errs))
}
