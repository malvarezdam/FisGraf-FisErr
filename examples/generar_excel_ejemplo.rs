//! Genera `examples/datos/ejemplo.xlsx`, el libro de ejemplo del proyecto.
//!
//! Uso: `cargo run --example generar_excel_ejemplo`

use rust_xlsxwriter::{Format, Workbook, XlsxError};

fn main() -> Result<(), XlsxError> {
    let mut book = Workbook::new();
    let bold = Format::new().set_bold();

    // Péndulo simple: T = 2π √(L/g), con g = 9,79 m/s² y ruido fijo.
    let sheet = book.add_worksheet().set_name("Pendulo")?;
    for (c, h) in ["L [m]", "T [s]", "σL [m]", "σT [s]"].iter().enumerate() {
        sheet.write_string_with_format(0, c as u16, *h, &bold)?;
    }
    let noise = [0.004, -0.006, 0.003, 0.007, -0.002, -0.005, 0.006, -0.003, 0.002];
    for (i, dn) in noise.iter().enumerate() {
        let l = 0.20 + 0.10 * i as f64;
        let t = 2.0 * std::f64::consts::PI * (l / 9.79).sqrt() + dn;
        let r = i as u32 + 1;
        sheet.write_number(r, 0, l)?;
        sheet.write_number(r, 1, (t * 1000.0).round() / 1000.0)?;
        sheet.write_number(r, 2, 0.001)?;
        sheet.write_number(r, 3, 0.005)?;
    }

    // Par de imanes (datos del gráfico de referencia), dos métodos de medida.
    let sheet = book.add_worksheet().set_name("Imanes")?;
    for (c, h) in ["x [cm]", "T Tracker [s]", "σT [s]", "T Cronómetro [s]"].iter().enumerate() {
        sheet.write_string_with_format(0, c as u16, *h, &bold)?;
    }
    let rows = [
        (25.80, 0.9600, 0.0030, 0.962),
        (26.60, 0.9993, 0.0030, 0.991),
        (28.20, 0.9860, 0.0030, 0.984),
        (29.25, 0.9987, 0.0030, 1.002),
    ];
    for (i, (x, t, s, t2)) in rows.iter().enumerate() {
        let r = i as u32 + 1;
        sheet.write_number(r, 0, *x)?;
        sheet.write_number(r, 1, *t)?;
        sheet.write_number(r, 2, *s)?;
        sheet.write_number(r, 3, *t2)?;
    }

    // Ley de Hooke con números escritos como texto con coma decimal,
    // tal como quedan a veces al copiar desde otra planilla.
    let sheet = book.add_worksheet().set_name("Hooke")?;
    sheet.write_string_with_format(0, 0, "m [kg]", &bold)?;
    sheet.write_string_with_format(0, 1, "Δx [m]", &bold)?;
    let hooke = [
        ("0,050", "0,0123"),
        ("0,100", "0,0249"),
        ("0,150", "0,0371"),
        ("0,200", "0,0502"),
        ("0,250", "0,0618"),
    ];
    for (i, (m, dx)) in hooke.iter().enumerate() {
        sheet.write_string(i as u32 + 1, 0, *m)?;
        sheet.write_string(i as u32 + 1, 1, *dx)?;
    }

    std::fs::create_dir_all("examples/datos").expect("crear carpeta de ejemplos");
    book.save("examples/datos/ejemplo.xlsx")?;
    println!("Escrito examples/datos/ejemplo.xlsx");
    Ok(())
}
