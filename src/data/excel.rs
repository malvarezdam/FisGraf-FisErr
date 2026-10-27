//! Lectura de libros Excel (.xlsx, .xlsm, .xls, .xlsb, .ods) con calamine.
//!
//! El libro se carga entero en memoria al abrirlo, así la interfaz puede
//! consultar celdas y vistas previas sin volver a leer el archivo.

use std::path::{Path, PathBuf};

use calamine::{Data, Range, Reader, open_workbook_auto};

use super::DataError;
use super::range::{CellRange, CellRef, Orientation};

struct Sheet {
    name: String,
    cells: Range<Data>,
}

pub struct Workbook {
    path: PathBuf,
    sheets: Vec<Sheet>,
}

impl Workbook {
    pub fn open(path: &Path) -> Result<Self, DataError> {
        let open_err = |e: &dyn std::fmt::Display| DataError::Open {
            path: path.to_owned(),
            reason: e.to_string(),
        };
        let mut book = open_workbook_auto(path).map_err(|e| open_err(&e))?;
        let mut sheets = Vec::new();
        for name in book.sheet_names() {
            let cells = book.worksheet_range(&name).map_err(|e| open_err(&e))?;
            sheets.push(Sheet { name, cells });
        }
        if sheets.is_empty() {
            return Err(open_err(&"el libro no tiene hojas"));
        }
        Ok(Self {
            path: path.to_owned(),
            sheets,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn sheet_names(&self) -> impl Iterator<Item = &str> {
        self.sheets.iter().map(|s| s.name.as_str())
    }

    /// Hoja por nombre; `None` = la primera.
    fn sheet(&self, name: Option<&str>) -> Result<&Sheet, DataError> {
        match name {
            None => Ok(&self.sheets[0]),
            Some(n) => self
                .sheets
                .iter()
                .find(|s| s.name == n)
                .or_else(|| self.sheets.iter().find(|s| s.name.eq_ignore_ascii_case(n)))
                .ok_or_else(|| DataError::SheetNotFound {
                    name: n.to_owned(),
                    available: self.sheet_names().collect::<Vec<_>>().join(", "),
                }),
        }
    }

    /// Valor de una celda; `None` si está fuera del área usada.
    pub fn cell(&self, sheet: Option<&str>, cell: CellRef) -> Result<Option<&Data>, DataError> {
        Ok(self.sheet(sheet)?.cells.get_value((cell.row, cell.col)))
    }

    /// Última fila y columna con datos (base 0). `None` si la hoja está vacía.
    pub fn used_end(&self, sheet: Option<&str>) -> Result<Option<CellRef>, DataError> {
        Ok(self.sheet(sheet)?.cells.end().map(|(r, c)| CellRef::new(r, c)))
    }

    /// Texto de las celdas desde `A1` hasta `max_rows × max_cols`, para la vista
    /// previa de la interfaz. Las filas y columnas se recortan al área usada.
    pub fn preview(
        &self,
        sheet: Option<&str>,
        max_rows: u32,
        max_cols: u32,
    ) -> Result<Vec<Vec<String>>, DataError> {
        let sheet = self.sheet(sheet)?;
        let Some((last_row, last_col)) = sheet.cells.end() else {
            return Ok(Vec::new());
        };
        let rows = (last_row + 1).min(max_rows);
        let cols = (last_col + 1).min(max_cols);
        Ok((0..rows)
            .map(|r| {
                (0..cols)
                    .map(|c| sheet.cells.get_value((r, c)).map_or_else(String::new, |d| d.to_string()))
                    .collect()
            })
            .collect())
    }

    /// Lee un rango de una columna o una fila como números.
    ///
    /// Las celdas vacías dan `None`. En un rango abierto (`B2:B`) se eliminan
    /// las celdas vacías del final. El texto que no es un número da error.
    pub fn read_vector(&self, range: &CellRange) -> Result<Vec<Option<f64>>, DataError> {
        let orientation = range.orientation()?;
        let sheet = self.sheet(range.sheet.as_deref())?;
        let last_row = sheet.cells.end().map_or(0, |(r, _)| r);
        let (lo, hi) = range.bounds(last_row);

        let cells: Vec<CellRef> = match orientation {
            Orientation::Column => (lo.row..=hi.row).map(|r| CellRef::new(r, lo.col)).collect(),
            Orientation::Row => (lo.col..=hi.col).map(|c| CellRef::new(lo.row, c)).collect(),
        };

        let mut values = cells
            .iter()
            .map(|&cell| {
                let data = sheet.cells.get_value((cell.row, cell.col)).unwrap_or(&Data::Empty);
                cell_to_number(data).map_err(|value| DataError::NotNumeric {
                    cell: format!("{}!{cell}", sheet.name),
                    value,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        if matches!(range.end, super::range::RangeEnd::OpenColumn(_)) {
            while values.last() == Some(&None) {
                values.pop();
            }
        }
        Ok(values)
    }

    /// Dirección legible de la `i`-ésima celda de un rango, p. ej. `Hoja1!B7`.
    pub fn nth_cell_name(&self, range: &CellRange, i: usize) -> String {
        let sheet = self.sheet(range.sheet.as_deref()).map_or("?", |s| s.name.as_str());
        let lo = range.bounds(0).0;
        let cell = match range.orientation() {
            Ok(Orientation::Row) => CellRef::new(lo.row, lo.col + i as u32),
            _ => CellRef::new(lo.row + i as u32, lo.col),
        };
        format!("{sheet}!{cell}")
    }
}

/// Convierte una celda en número. Vacía → `Ok(None)`; no numérica → `Err(texto)`.
fn cell_to_number(data: &Data) -> Result<Option<f64>, String> {
    match data {
        Data::Empty => Ok(None),
        Data::Int(i) => Ok(Some(*i as f64)),
        Data::Float(f) => Ok(Some(*f)),
        // Fecha u hora: número de serie de Excel (días desde 1899-12-30).
        Data::DateTime(dt) => Ok(Some(dt.as_f64())),
        Data::String(s) => {
            if s.trim().is_empty() {
                Ok(None)
            } else {
                parse_number(s).map(Some).ok_or_else(|| s.clone())
            }
        }
        other => Err(other.to_string()),
    }
}

/// Interpreta un número escrito como texto. Acepta coma decimal (`0,96`),
/// separadores de miles (`1.234,5`, `1,234.5`, `1 234,5`), signo menos
/// tipográfico (`−`) y notación científica (`1,5e-3`, `1.5E3`).
pub fn parse_number(s: &str) -> Option<f64> {
    let mut t: String = s
        .trim()
        .chars()
        .filter(|c| !matches!(c, ' ' | '\u{a0}' | '\u{202f}' | '\''))
        .map(|c| if c == '−' { '-' } else { c })
        .collect();

    match (t.rfind(','), t.rfind('.')) {
        // El separador que aparece más a la derecha es el decimal.
        (Some(comma), Some(dot)) if comma > dot => {
            t = t.replace('.', "").replace(',', ".");
        }
        (Some(_), Some(_)) => t = t.replace(',', ""),
        (Some(_), None) => {
            if t.matches(',').count() > 1 {
                t = t.replace(',', "");
            } else {
                t = t.replace(',', ".");
            }
        }
        _ => {}
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_as_text() {
        let cases = [
            ("0,96", 0.96),
            ("0.96", 0.96),
            ("  -3,5 ", -3.5),
            ("−2", -2.0),
            ("1.234,5", 1234.5),
            ("1,234.5", 1234.5),
            ("1 234,5", 1234.5),
            ("1,5e-3", 1.5e-3),
            ("2E3", 2000.0),
            ("1,000,000", 1e6),
        ];
        for (s, v) in cases {
            let got = parse_number(s).unwrap_or_else(|| panic!("{s}"));
            assert!((got - v).abs() < 1e-12, "{s} → {got}");
        }
        for s in ["abc", "", "1,2,3.4.5", "NaN", "inf"] {
            assert_eq!(parse_number(s), None, "{s}");
        }
    }

    #[test]
    fn cell_types() {
        assert_eq!(cell_to_number(&Data::Empty), Ok(None));
        assert_eq!(cell_to_number(&Data::Int(3)), Ok(Some(3.0)));
        assert_eq!(cell_to_number(&Data::String(" ".into())), Ok(None));
        assert_eq!(cell_to_number(&Data::String("x".into())), Err("x".into()));
        assert!(cell_to_number(&Data::Bool(true)).is_err());
    }
}
