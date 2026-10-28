//! Pestaña Datos: archivo Excel, hojas y selección de rangos con el ratón.

use egui::{Color32, RichText, Sense, Ui};
use egui_extras::{Column, TableBuilder};

use crate::config::ErrorSpec;
use crate::data::range::column_to_letters;
use crate::data::{CellRange, CellRef, RangeEnd};
use crate::gui::FisGrafApp;
use crate::gui::widgets::ERROR_COLOR;

/// Límites de la vista de celdas (el Excel completo se lee igual).
const MAX_ROWS: u32 = 1000;
const MAX_COLS: u32 = 40;

const SELECTION: Color32 = Color32::from_rgba_premultiplied(40, 80, 170, 110);
const X_FILL: Color32 = Color32::from_rgba_premultiplied(15, 60, 90, 70);
const Y_FILL: Color32 = Color32::from_rgba_premultiplied(100, 20, 20, 70);
const ERR_FILL: Color32 = Color32::from_rgba_premultiplied(60, 60, 60, 50);

/// Destino al que se asigna la selección.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    X,
    Y,
    XError,
    YError,
}

impl FisGrafApp {
    /// Rango seleccionado en la tabla, si hay.
    pub fn selection_range(&self) -> Option<CellRange> {
        let (a, b) = (self.selection.anchor?, self.selection.cursor.or(self.selection.anchor)?);
        let start = CellRef::new(a.row.min(b.row), a.col.min(b.col));
        let end = CellRef::new(a.row.max(b.row), a.col.max(b.col));
        let end = if self.open_ended && start.col == end.col { RangeEnd::OpenColumn(end.col) } else { RangeEnd::Cell(end) };
        Some(CellRange { sheet: self.sheet.clone(), start, end })
    }

    /// Asigna la selección a la serie elegida (la crea si no hay ninguna).
    pub fn assign_selection(&mut self, target: Target) {
        let Some(range) = self.selection_range() else { return };
        if let Err(e) = range.orientation() {
            self.error(format!("No se puede asignar: {e}"));
            return;
        }
        if self.selected_series.is_none_or(|id| self.project.series_by_id(id).is_none()) {
            self.selected_series = Some(self.project.add_series());
        }
        let id = self.selected_series.expect("serie elegida");
        let s = self.project.series.iter_mut().find(|s| s.id == id).expect("existe");
        let text = range.to_string();
        match target {
            Target::X => s.x_range = text,
            Target::Y => s.y_range = text,
            Target::XError => s.x_error = ErrorSpec::Range(text),
            Target::YError => s.y_error = ErrorSpec::Range(text),
        }
    }

    /// Crea una serie a partir de una selección de 2 o 3 columnas (o filas):
    /// x, y y opcionalmente σy. El nombre sale del encabezado de y si existe.
    pub fn create_series_from_selection(&mut self) -> bool {
        let Some(range) = self.selection_range() else { return false };
        let last_row = self.workbook.as_ref().and_then(|b| b.used_end(range.sheet.as_deref()).ok().flatten()).map_or(0, |c| c.row);
        let (lo, hi) = range.bounds(last_row);
        let by_columns = hi.col > lo.col;
        let count = if by_columns { hi.col - lo.col + 1 } else { hi.row - lo.row + 1 };
        if count < 2 || (!by_columns && hi.col == lo.col) {
            self.error("Selecciona al menos dos columnas contiguas (x, y y opcionalmente σy).");
            return false;
        }
        let make = |k: u32| -> String {
            let (start, end) = if by_columns {
                let c = lo.col + k;
                let end = if self.open_ended { RangeEnd::OpenColumn(c) } else { RangeEnd::Cell(CellRef::new(hi.row, c)) };
                (CellRef::new(lo.row, c), end)
            } else {
                let r = lo.row + k;
                (CellRef::new(r, lo.col), RangeEnd::Cell(CellRef::new(r, hi.col)))
            };
            CellRange { sheet: range.sheet.clone(), start, end }.to_string()
        };
        // Encabezado: celda justo antes del primer dato de la columna (o fila) de y.
        let header = self.workbook.as_ref().and_then(|b| {
            let cell = if by_columns && lo.row > 0 {
                CellRef::new(lo.row - 1, lo.col + 1)
            } else if !by_columns && lo.col > 0 {
                CellRef::new(lo.row + 1, lo.col - 1)
            } else {
                return None;
            };
            match b.cell(range.sheet.as_deref(), cell).ok().flatten() {
                Some(calamine::Data::String(s)) if !s.trim().is_empty() => Some(s.trim().to_owned()),
                _ => None,
            }
        });

        let id = self.project.add_series();
        let s = self.project.series.last_mut().expect("recién creada");
        s.x_range = make(0);
        s.y_range = make(1);
        if count >= 3 {
            s.y_error = ErrorSpec::Range(make(2));
        }
        if let Some(h) = header {
            s.label = h;
        }
        self.selected_series = Some(id);
        true
    }

    pub(crate) fn data_panel(&mut self, ui: &mut Ui) {
        ui.heading("Archivo Excel");
        ui.horizontal(|ui| match &self.project.excel_path {
            Some(p) => {
                let name = p.file_name().map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into_owned());
                ui.label(RichText::new(name).strong()).on_hover_text(p.display().to_string());
            }
            None => {
                ui.weak("Ningún archivo abierto");
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Abrir Excel…").clicked()
                && let Some(p) = rfd::FileDialog::new().add_filter("Hojas de cálculo", &["xlsx", "xlsm", "xls", "xlsb", "ods"]).pick_file()
            {
                self.project.excel_path = Some(p);
            }
            if self.project.excel_path.is_some() && ui.button("Recargar").on_hover_text("Volver a leer el archivo si lo editaste").clicked() {
                self.reload_workbook();
            }
        });
        if let Some(e) = &self.workbook_error {
            ui.colored_label(ERROR_COLOR, e);
        }
        let Some(book) = self.workbook.clone() else {
            ui.add_space(10.0);
            ui.label("Abre un archivo .xlsx, .xls u .ods con tus mediciones.");
            return;
        };

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Hoja:");
            let current = self.sheet.clone().unwrap_or_default();
            let r = egui::ComboBox::from_id_salt("hoja").selected_text(&current).show_ui(ui, |ui| {
                for name in book.sheet_names() {
                    if ui.selectable_label(current == name, name).clicked() && current != name {
                        self.sheet = Some(name.to_owned());
                        self.selection = Default::default();
                    }
                }
            });
            crate::gui::widgets::accessible(&r.response, egui::Role::ComboBox, "Hoja");
        });

        ui.weak("Clic en una celda y Mayús+clic en otra para seleccionar un rango. Clic en la letra de una columna la selecciona desde la fila 2.");
        let sel = self.selection_range();
        ui.horizontal(|ui| {
            ui.label("Selección:");
            match &sel {
                Some(r) => ui.monospace(r.to_string()),
                None => ui.weak("ninguna"),
            };
        });
        ui.checkbox(&mut self.open_ended, "Hasta el final de la columna (B2:B)")
            .on_hover_text("El rango crece solo si agregas filas al Excel");

        let series_name = self.selected_series.and_then(|id| self.project.series_by_id(id)).map(|s| s.label.clone());
        ui.horizontal_wrapped(|ui| {
            ui.label(match &series_name {
                Some(n) => format!("Asignar a «{n}»:"),
                None => "Asignar a una serie nueva:".into(),
            });
            for (target, text, tip) in [
                (Target::X, "→ x", "Variable independiente"),
                (Target::Y, "→ y", "Variable dependiente"),
                (Target::XError, "→ σx", "Errores de x"),
                (Target::YError, "→ σy", "Errores de y"),
            ] {
                if ui.add_enabled(sel.is_some(), egui::Button::new(text)).on_hover_text(tip).clicked() {
                    self.assign_selection(target);
                }
            }
        });
        if ui
            .add_enabled(sel.is_some(), egui::Button::new("Crear serie con la selección"))
            .on_hover_text("2 columnas: x, y · 3 columnas: x, y, σy")
            .clicked()
            && self.create_series_from_selection()
        {
            self.info("Serie creada a partir de la selección");
        }
        ui.separator();
        self.cell_table(ui, &book);
    }

    fn cell_table(&mut self, ui: &mut Ui, book: &crate::data::Workbook) {
        let sheet = self.sheet.clone().unwrap_or_default();
        if self.preview_cache.as_ref().is_none_or(|(s, _)| *s != sheet) {
            let cells = book.preview(Some(&sheet), MAX_ROWS, MAX_COLS).unwrap_or_default();
            self.preview_cache = Some((sheet.clone(), cells));
        }
        let cells = &self.preview_cache.as_ref().expect("recién calculada").1;
        if cells.is_empty() {
            ui.weak("La hoja está vacía.");
            return;
        }
        let rows = cells.len();
        let cols = cells.iter().map(Vec::len).max().unwrap_or(0);
        let last_row = book.used_end(Some(&sheet)).ok().flatten().map_or(0, |c| c.row);

        // Rangos de la serie elegida en esta hoja, para resaltarlos.
        let first_sheet = book.sheet_names().next().unwrap_or_default().to_owned();
        let mut marks: Vec<((CellRef, CellRef), Color32)> = Vec::new();
        if let Some(s) = self.selected_series.and_then(|id| self.project.series_by_id(id)) {
            let mut add = |text: &str, color| {
                if let Ok(r) = text.parse::<CellRange>()
                    && r.sheet.as_deref().unwrap_or(&first_sheet).eq_ignore_ascii_case(&sheet)
                {
                    marks.push((r.bounds(last_row), color));
                }
            };
            add(&s.x_range, X_FILL);
            add(&s.y_range, Y_FILL);
            if let ErrorSpec::Range(r) = &s.x_error {
                add(r, ERR_FILL);
            }
            if let ErrorSpec::Range(r) = &s.y_error {
                add(r, ERR_FILL);
            }
        }
        let selected = self.selection_range().map(|r| r.bounds(last_row));
        let inside = |b: &(CellRef, CellRef), r: u32, c: u32| (b.0.row..=b.1.row).contains(&r) && (b.0.col..=b.1.col).contains(&c);

        let mut clicked: Option<(CellRef, bool)> = None;
        let mut header_clicked: Option<(u32, bool)> = None;
        egui::ScrollArea::horizontal().id_salt("celdas_h").show(ui, |ui| {
            let mut table = TableBuilder::new(ui)
                .id_salt(("celdas", &sheet))
                .striped(true)
                .vscroll(true)
                .max_scroll_height(420.0)
                .column(Column::exact(34.0));
            for _ in 0..cols {
                table = table.column(Column::initial(68.0).at_least(36.0).clip(true).resizable(true));
            }
            table
                .header(20.0, |mut h| {
                    h.col(|_| {});
                    for c in 0..cols as u32 {
                        h.col(|ui| {
                            let r = ui.interact(ui.max_rect(), ui.scope_id().with(("col", c)), Sense::click());
                            ui.strong(column_to_letters(c));
                            if r.clicked() {
                                header_clicked = Some((c, ui.input(|i| i.modifiers.shift)));
                            }
                        });
                    }
                })
                .body(|body| {
                    body.rows(18.0, rows, |mut row| {
                        let r = row.index() as u32;
                        row.col(|ui| {
                            ui.weak((r + 1).to_string());
                        });
                        for c in 0..cols as u32 {
                            row.col(|ui| {
                                let rect = ui.max_rect();
                                let fill = if selected.as_ref().is_some_and(|b| inside(b, r, c)) {
                                    Some(SELECTION)
                                } else {
                                    marks.iter().find(|(b, _)| inside(b, r, c)).map(|(_, col)| *col)
                                };
                                if let Some(f) = fill {
                                    ui.painter().rect_filled(rect, 0.0, f);
                                }
                                let resp = ui.interact(rect, ui.scope_id().with(("celda", r, c)), Sense::click());
                                let text = cells[r as usize].get(c as usize).map_or("", String::as_str);
                                ui.label(text);
                                if resp.clicked() {
                                    clicked = Some((CellRef::new(r, c), ui.input(|i| i.modifiers.shift)));
                                }
                            });
                        }
                    });
                });
        });

        if let Some((cell, shift)) = clicked {
            if shift && self.selection.anchor.is_some() {
                self.selection.cursor = Some(cell);
            } else {
                self.selection.anchor = Some(cell);
                self.selection.cursor = Some(cell);
            }
        }
        if let Some((c, shift)) = header_clicked {
            let start_row = 1.min(last_row);
            if shift && let Some(a) = self.selection.anchor {
                self.selection.cursor = Some(CellRef::new(last_row, c));
                self.selection.anchor = Some(CellRef::new(start_row, a.col));
            } else {
                self.selection.anchor = Some(CellRef::new(start_row, c));
                self.selection.cursor = Some(CellRef::new(last_row, c));
            }
        }
        ui.horizontal(|ui| {
            for (color, text) in [(X_FILL, "x"), (Y_FILL, "y"), (ERR_FILL, "errores"), (SELECTION, "selección")] {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
                ui.painter().rect_filled(rect, 2.0, color);
                ui.weak(text);
            }
        });
    }
}
