//! Pruebas de lectura de Excel con libros generados al vuelo.

use std::path::PathBuf;

use fisgraf::config::{ErrorSpec, SeriesConfig};
use fisgraf::data::{DataError, Workbook, load_series};
use rust_xlsxwriter::Workbook as Writer;

/// Libro temporal que se borra al terminar la prueba.
struct TempBook(PathBuf);

impl Drop for TempBook {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn build_book(tag: &str) -> TempBook {
    let mut w = Writer::new();

    let s = w.add_worksheet().set_name("Datos").unwrap();
    s.write_string(0, 0, "x").unwrap();
    s.write_string(0, 1, "y").unwrap();
    s.write_string(0, 2, "sy").unwrap();
    // Fila 4 (índice 3) sin y: debe descartarse.
    let rows: [(f64, Option<f64>, f64); 5] = [
        (1.0, Some(2.1), 0.1),
        (2.0, Some(3.9), 0.2),
        (3.0, None, 0.1),
        (4.0, Some(8.2), 0.3),
        (5.0, Some(9.8), 0.2),
    ];
    for (i, (x, y, sy)) in rows.iter().enumerate() {
        let r = i as u32 + 1;
        s.write_number(r, 0, *x).unwrap();
        if let Some(y) = y {
            s.write_number(r, 1, *y).unwrap();
        }
        s.write_number(r, 2, *sy).unwrap();
    }
    s.write_string(10, 0, "texto").unwrap();

    let s = w.add_worksheet().set_name("Mi hoja").unwrap();
    // Datos en filas y como texto con coma decimal.
    for (c, v) in ["0,5", "1,5", "2,5"].iter().enumerate() {
        s.write_string(2, c as u16 + 1, *v).unwrap();
    }
    for (c, v) in [10.0, 20.0, 30.0].iter().enumerate() {
        s.write_number(3, c as u16 + 1, *v).unwrap();
    }

    let path = std::env::temp_dir().join(format!("fisgraf_{tag}_{}.xlsx", std::process::id()));
    w.save(&path).unwrap();
    TempBook(path)
}

fn series(x: &str, y: &str) -> SeriesConfig {
    SeriesConfig {
        label: "Prueba".into(),
        x_range: x.into(),
        y_range: y.into(),
        ..Default::default()
    }
}

#[test]
fn sheets_and_preview() {
    let tmp = build_book("sheets");
    let book = Workbook::open(&tmp.0).unwrap();
    assert_eq!(book.sheet_names().collect::<Vec<_>>(), ["Datos", "Mi hoja"]);

    let preview = book.preview(None, 3, 10).unwrap();
    assert_eq!(preview.len(), 3);
    assert_eq!(preview[0], ["x", "y", "sy"]);
    assert_eq!(preview[1][1], "2.1");
}

#[test]
fn column_series_with_error_column() {
    let tmp = build_book("cols");
    let book = Workbook::open(&tmp.0).unwrap();
    let mut cfg = series("Datos!A2:A6", "Datos!B2:B6");
    cfg.y_error = ErrorSpec::Range("Datos!C2:C6".into());
    cfg.x_error = ErrorSpec::Constant(0.05);

    let d = load_series(&book, &cfg).unwrap();
    assert_eq!(d.x, [1.0, 2.0, 4.0, 5.0]);
    assert_eq!(d.y, [2.1, 3.9, 8.2, 9.8]);
    assert_eq!(d.y_err.unwrap(), [0.1, 0.2, 0.3, 0.2]);
    assert_eq!(d.x_err.unwrap(), [0.05; 4]);
    assert_eq!(d.skipped, [2]);
}

#[test]
fn open_ranges_and_default_sheet() {
    let tmp = build_book("open");
    let book = Workbook::open(&tmp.0).unwrap();
    // Sin hoja = primera hoja. B2:B termina en la última fila con datos de B.
    let mut cfg = series("A2:A6", "B2:B");
    cfg.y_error = ErrorSpec::Percent(10.0);
    let d = load_series(&book, &cfg).unwrap();
    assert_eq!(d.len(), 4);
    let sy = d.y_err.unwrap();
    assert!((sy[0] - 0.21).abs() < 1e-12);
}

#[test]
fn row_series_with_text_numbers() {
    let tmp = build_book("rows");
    let book = Workbook::open(&tmp.0).unwrap();
    let mut cfg = series("'Mi hoja'!B3:D3", "'mi hoja'!B4:D4");
    cfg.x_error = ErrorSpec::Resolution(0.1);
    let d = load_series(&book, &cfg).unwrap();
    assert_eq!(d.x, [0.5, 1.5, 2.5]);
    assert_eq!(d.y, [10.0, 20.0, 30.0]);
    let sx = d.x_err.unwrap()[0];
    assert!((sx - 0.1 / (2.0 * 3f64.sqrt())).abs() < 1e-15);
}

#[test]
fn readable_errors() {
    let tmp = build_book("errors");
    let book = Workbook::open(&tmp.0).unwrap();
    let msg = |x: &str, y: &str| load_series(&book, &series(x, y)).unwrap_err().to_string();

    let m = msg("Datos!A2:A11", "Datos!B2:B11");
    assert!(m.contains("Datos!A11") && m.contains("texto"), "{m}");

    let m = msg("Datos!A2:A6", "Datos!B2:B5");
    assert!(m.contains("distinto número"), "{m}");

    let m = msg("Otra!A2:A6", "Datos!B2:B6");
    assert!(m.contains("no existe la hoja") && m.contains("Mi hoja"), "{m}");

    let m = msg("Datos!A2:B6", "Datos!B2:B6");
    assert!(m.contains("una sola columna"), "{m}");

    let m = msg("", "Datos!B2:B6");
    assert!(m.starts_with("serie «Prueba»"), "{m}");

    let mut cfg = series("Datos!A2:A6", "Datos!B2:B6");
    cfg.y_error = ErrorSpec::Constant(-1.0);
    assert!(load_series(&book, &cfg).unwrap_err().to_string().contains("≥ 0"));

    // Error vacío en una fila válida: se informa la celda.
    cfg.y_error = ErrorSpec::Range("Datos!D2:D6".into());
    let m = load_series(&book, &cfg).unwrap_err().to_string();
    assert!(m.contains("Datos!D2"), "{m}");
}

#[test]
fn open_missing_file() {
    let err = Workbook::open(std::path::Path::new("no_existe.xlsx")).err().unwrap();
    assert!(matches!(err, DataError::Open { .. }));
}

#[test]
fn example_workbook_loads() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/datos/ejemplo.xlsx");
    let book = Workbook::open(&path).unwrap();
    let mut cfg = series("Pendulo!A2:A", "Pendulo!B2:B");
    cfg.y_error = ErrorSpec::Range("Pendulo!D2:D".into());
    assert_eq!(load_series(&book, &cfg).unwrap().len(), 9);

    let d = load_series(&book, &series("Hooke!A2:A6", "Hooke!B2:B6")).unwrap();
    assert_eq!(d.x[0], 0.05);
    assert_eq!(d.y[4], 0.0618);
}
