//! Referencias de celdas en notación A1: `B2`, `B2:B20`, `Hoja1!B2:B20`,
//! `'Mi hoja'!B2:B20` y rangos abiertos `B2:B` (hasta el final de los datos).

use std::fmt;
use std::str::FromStr;

/// Celda con fila y columna base 0 (`A1` = `(0, 0)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellRef {
    pub row: u32,
    pub col: u32,
}

impl CellRef {
    pub fn new(row: u32, col: u32) -> Self {
        Self { row, col }
    }
}

impl fmt::Display for CellRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", column_to_letters(self.col), self.row + 1)
    }
}

impl FromStr for CellRef {
    type Err = RangeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let split = s.find(|c: char| c.is_ascii_digit()).unwrap_or(s.len());
        let (letters, digits) = s.split_at(split);
        let col = letters_to_column(letters).ok_or_else(|| RangeError::Cell(s.to_owned()))?;
        let row: u32 = digits.parse().map_err(|_| RangeError::Cell(s.to_owned()))?;
        if row == 0 {
            return Err(RangeError::Cell(s.to_owned()));
        }
        Ok(Self { row: row - 1, col })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RangeError {
    #[error("el rango está vacío")]
    Empty,
    #[error("celda inválida `{0}`: se espera algo como `B2`")]
    Cell(String),
    #[error("rango inválido `{0}`: se espera algo como `Hoja1!B2:B20`")]
    Range(String),
    #[error("el rango `{0}` debe ser una sola columna o una sola fila")]
    NotVector(String),
}

/// Rango rectangular de celdas, opcionalmente en una hoja concreta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellRange {
    /// `None` = hoja por defecto (la primera del libro).
    pub sheet: Option<String>,
    pub start: CellRef,
    pub end: RangeEnd,
}

/// Final de un rango.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeEnd {
    Cell(CellRef),
    /// Columna sin fila final (`B2:B`): llega hasta la última fila con datos.
    OpenColumn(u32),
}

/// Orientación de un rango de una dimensión.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Column,
    Row,
}

impl CellRange {
    /// Comprueba que el rango sea una columna o una fila y devuelve su orientación.
    /// Una sola celda se considera columna.
    pub fn orientation(&self) -> Result<Orientation, RangeError> {
        match self.end {
            RangeEnd::OpenColumn(col) if col == self.start.col => Ok(Orientation::Column),
            RangeEnd::Cell(end) if end.col == self.start.col => Ok(Orientation::Column),
            RangeEnd::Cell(end) if end.row == self.start.row => Ok(Orientation::Row),
            _ => Err(RangeError::NotVector(self.to_string())),
        }
    }

    /// Fila y columna mínimas y máximas. Un rango abierto termina en `last_row`.
    pub fn bounds(&self, last_row: u32) -> (CellRef, CellRef) {
        let end = match self.end {
            RangeEnd::Cell(c) => c,
            RangeEnd::OpenColumn(col) => CellRef::new(last_row.max(self.start.row), col),
        };
        (
            CellRef::new(self.start.row.min(end.row), self.start.col.min(end.col)),
            CellRef::new(self.start.row.max(end.row), self.start.col.max(end.col)),
        )
    }
}

impl fmt::Display for CellRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(sheet) = &self.sheet {
            if sheet.chars().all(|c| c.is_alphanumeric() || c == '_') {
                write!(f, "{sheet}!")?;
            } else {
                write!(f, "'{}'!", sheet.replace('\'', "''"))?;
            }
        }
        write!(f, "{}", self.start)?;
        match self.end {
            RangeEnd::Cell(end) if end == self.start => Ok(()),
            RangeEnd::Cell(end) => write!(f, ":{end}"),
            RangeEnd::OpenColumn(col) => write!(f, ":{}", column_to_letters(col)),
        }
    }
}

impl FromStr for CellRange {
    type Err = RangeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(RangeError::Empty);
        }
        let bad = || RangeError::Range(s.to_owned());

        let (sheet, cells) = match s.rfind('!') {
            Some(i) => (Some(parse_sheet_name(&s[..i]).ok_or_else(bad)?), &s[i + 1..]),
            None => (None, s),
        };
        let cells = cells.replace('$', "");

        let (start, end) = match cells.split_once(':') {
            None => {
                let c: CellRef = cells.parse()?;
                (c, RangeEnd::Cell(c))
            }
            Some((a, b)) => {
                let start: CellRef = a.parse()?;
                let b = b.trim();
                let end = if b.chars().all(|c| c.is_ascii_alphabetic()) {
                    RangeEnd::OpenColumn(letters_to_column(b).ok_or_else(bad)?)
                } else {
                    RangeEnd::Cell(b.parse()?)
                };
                (start, end)
            }
        };
        Ok(Self { sheet, start, end })
    }
}

fn parse_sheet_name(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let name = match raw.strip_prefix('\'') {
        Some(rest) => rest.strip_suffix('\'')?.replace("''", "'"),
        None => raw.to_owned(),
    };
    (!name.is_empty()).then_some(name)
}

/// `A` → 0, `Z` → 25, `AA` → 26.
pub fn letters_to_column(letters: &str) -> Option<u32> {
    if letters.is_empty() || letters.len() > 3 || !letters.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let n = letters
        .chars()
        .fold(0u32, |acc, c| acc * 26 + (c.to_ascii_uppercase() as u32 - 'A' as u32 + 1));
    Some(n - 1)
}

/// 0 → `A`, 25 → `Z`, 26 → `AA`.
pub fn column_to_letters(col: u32) -> String {
    let mut n = col + 1;
    let mut out = Vec::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        out.push(b'A' + rem as u8);
        n = (n - 1) / 26;
    }
    out.reverse();
    String::from_utf8(out).expect("solo letras ASCII")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns() {
        for (s, n) in [("A", 0), ("Z", 25), ("AA", 26), ("AZ", 51), ("BA", 52), ("XFD", 16383)] {
            assert_eq!(letters_to_column(s), Some(n), "{s}");
            assert_eq!(column_to_letters(n), s);
        }
        assert_eq!(letters_to_column("b"), Some(1));
        assert_eq!(letters_to_column(""), None);
        assert_eq!(letters_to_column("A1"), None);
    }

    #[test]
    fn cells() {
        assert_eq!("B2".parse::<CellRef>().unwrap(), CellRef::new(1, 1));
        assert_eq!("aa10".parse::<CellRef>().unwrap(), CellRef::new(9, 26));
        assert!("B0".parse::<CellRef>().is_err());
        assert!("2B".parse::<CellRef>().is_err());
        assert!("B".parse::<CellRef>().is_err());
    }

    #[test]
    fn ranges() {
        let r: CellRange = "Hoja1!B2:B20".parse().unwrap();
        assert_eq!(r.sheet.as_deref(), Some("Hoja1"));
        assert_eq!(r.start, CellRef::new(1, 1));
        assert_eq!(r.end, RangeEnd::Cell(CellRef::new(19, 1)));
        assert_eq!(r.orientation(), Ok(Orientation::Column));

        let r: CellRange = "'Datos del péndulo'!$C$3:$H$3".parse().unwrap();
        assert_eq!(r.sheet.as_deref(), Some("Datos del péndulo"));
        assert_eq!(r.orientation(), Ok(Orientation::Row));

        let r: CellRange = "D5:D".parse().unwrap();
        assert_eq!(r.sheet, None);
        assert_eq!(r.end, RangeEnd::OpenColumn(3));
        assert_eq!(r.bounds(40).1, CellRef::new(40, 3));

        let r: CellRange = "B2:C5".parse().unwrap();
        assert!(r.orientation().is_err());

        assert_eq!("".parse::<CellRange>(), Err(RangeError::Empty));
        assert!("Hoja1!".parse::<CellRange>().is_err());
        assert!("!B2".parse::<CellRange>().is_err());
    }

    #[test]
    fn display_roundtrip() {
        for s in ["Hoja1!B2:B20", "'Mi hoja'!A1:A5", "'It''s'!C3", "B2:B", "C3:H3"] {
            let r: CellRange = s.parse().unwrap();
            assert_eq!(r.to_string(), s);
        }
    }
}
