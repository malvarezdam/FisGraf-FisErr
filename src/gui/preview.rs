//! Vista rápida con egui_plot: se actualiza al instante mientras se edita.
//! Es una aproximación; el gráfico final lo compone LaTeX.

use egui::Ui;
use egui_plot::{Corner, FilledArea, Legend, Line, LineStyle, MarkerShape, Plot, PlotPoints, Points};

use crate::config::{AxisScale, DashPattern, LegendPosition, Marker, ProjectConfig};
use crate::gui::text::latex_to_plain;
use crate::gui::widgets::to_color32;
use crate::gui::worker::Output;

/// Puntos por pt tipográfico en pantalla (aproximado).
const PX_PER_PT: f32 = 1.5;

fn dash_style(d: DashPattern) -> LineStyle {
    match d {
        DashPattern::Solid => LineStyle::Solid,
        DashPattern::Dashed => LineStyle::Dashed { length: 8.0 },
        DashPattern::DenselyDashed => LineStyle::Dashed { length: 5.0 },
        DashPattern::LooselyDashed => LineStyle::Dashed { length: 12.0 },
        DashPattern::Dotted => LineStyle::Dotted { spacing: 8.0 },
        DashPattern::DenselyDotted => LineStyle::Dotted { spacing: 5.0 },
        DashPattern::DashDot => LineStyle::Dashed { length: 6.0 },
    }
}

fn shape(m: Marker) -> Option<MarkerShape> {
    Some(match m {
        Marker::Circle => MarkerShape::Circle,
        Marker::Square => MarkerShape::Square,
        Marker::Triangle => MarkerShape::Up,
        Marker::Diamond => MarkerShape::Diamond,
        Marker::Cross => MarkerShape::Cross,
        Marker::Plus => MarkerShape::Plus,
        Marker::Star => MarkerShape::Asterisk,
        Marker::None => return None,
    })
}

fn corner(p: LegendPosition) -> Corner {
    match p {
        LegendPosition::NorthEast | LegendPosition::OuterEast => Corner::RightTop,
        LegendPosition::NorthWest => Corner::LeftTop,
        LegendPosition::SouthEast | LegendPosition::Below => Corner::RightBottom,
        LegendPosition::SouthWest => Corner::LeftBottom,
        LegendPosition::Custom { x, y } => match (x > 0.5, y > 0.5) {
            (true, true) => Corner::RightTop,
            (false, true) => Corner::LeftTop,
            (true, false) => Corner::RightBottom,
            (false, false) => Corner::LeftBottom,
        },
    }
}

/// Divide una curva en tramos de puntos finitos.
fn segments(points: impl Iterator<Item = [f64; 2]>) -> Vec<Vec<[f64; 2]>> {
    let mut out = vec![Vec::new()];
    for p in points {
        if p[0].is_finite() && p[1].is_finite() {
            out.last_mut().expect("no vacío").push(p);
        } else if !out.last().expect("no vacío").is_empty() {
            out.push(Vec::new());
        }
    }
    out.retain(|s| !s.is_empty());
    out
}

pub fn quick_preview(ui: &mut Ui, project: &ProjectConfig, out: Option<&Output>, busy: bool) {
    ui.weak("Aproximada: el gráfico final se compone con LaTeX (pestaña «Vista LaTeX»).");
    let Some(out) = out else {
        ui.add_space(30.0);
        if busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Calculando…");
            });
        } else {
            ui.label("Abre un Excel en la pestaña Datos y define una serie para ver el gráfico.");
        }
        return;
    };
    let scene = &out.scene;
    if scene.points.is_empty() && scene.curves.is_empty() {
        ui.add_space(30.0);
        ui.label("Todavía no hay nada que dibujar: revisa las series y los mensajes de abajo.");
        return;
    }

    let logx = project.x_axis.scale == AxisScale::Log10;
    let logy = project.y_axis.scale == AxisScale::Log10;
    let tx = move |x: f64| if logx { x.log10() } else { x };
    let ty = move |y: f64| if logy { y.log10() } else { y };
    let label = |text: &str, log: bool| {
        let t = latex_to_plain(text);
        if log { format!("{t}  (log₁₀)") } else { t }
    };

    let (x0, x1) = (tx(scene.x_range.0), tx(scene.x_range.1));
    let ylim = (project.y_axis.min.map(ty), project.y_axis.max.map(ty));
    // El id incluye los límites: si cambian, la vista se reinicia a ellos.
    let key = (x0.to_bits(), x1.to_bits(), ylim.0.map(f64::to_bits), ylim.1.map(f64::to_bits), logx, logy);
    let fig = &project.figure;
    let mut plot = Plot::new(("vista_rapida", key))
        .x_axis_label(label(&project.x_axis.label, logx))
        .y_axis_label(label(&project.y_axis.label, logy))
        .default_x_bounds(x0, x1)
        .show_grid(fig.grid.major || fig.grid.minor);
    if fig.legend.show {
        plot = plot.legend(Legend::default().position(corner(fig.legend.position)));
    }
    if let (Some(a), Some(b)) = ylim {
        plot = plot.default_y_bounds(a, b);
    }

    // La aplicación es oscura, pero el gráfico se ve claro, como quedará impreso.
    let mut ui = ui.new_child(egui::UiBuilder::new());
    let ui = &mut ui;
    *ui.visuals_mut() = egui::Visuals::light();
    let rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(rect, 4.0, egui::Color32::WHITE);

    plot.show(ui, |pui| {
        for (i, c) in scene.curves.iter().enumerate() {
            if let Some(b) = &c.band {
                let pts: Vec<_> = b.points.iter().filter(|p| p.1.is_finite() && p.2.is_finite()).collect();
                let xs: Vec<f64> = pts.iter().map(|p| tx(p.0)).collect();
                let lo: Vec<f64> = pts.iter().map(|p| ty(p.1)).collect();
                let hi: Vec<f64> = pts.iter().map(|p| ty(p.2)).collect();
                let fill = to_color32(c.color).gamma_multiply(b.opacity.clamp(0.05, 1.0));
                // Mismo nombre que su curva: comparten la entrada de la leyenda.
                let name = c.legend.as_deref().map_or_else(|| format!("##curva{i}"), latex_to_plain);
                pui.add(FilledArea::new(name, &xs, &lo, &hi).id(("banda", i)).fill_color(fill).allow_hover(false));
            }
        }
        for (i, c) in scene.curves.iter().enumerate() {
            let name = c.legend.as_deref().map_or_else(|| format!("##curva{i}"), latex_to_plain);
            let color = to_color32(c.color).gamma_multiply(c.line.opacity);
            for (j, seg) in segments(c.points.iter().map(|(x, y)| [tx(*x), ty(*y)])).into_iter().enumerate() {
                pui.line(
                    Line::new(name.clone(), PlotPoints::from(seg))
                        .id(("curva", i, j))
                        .color(color)
                        .width(c.line.width_pt * PX_PER_PT)
                        .style(dash_style(c.line.dash)),
                );
            }
        }
        for (i, p) in scene.points.iter().enumerate() {
            let name = p.legend.as_deref().map_or_else(|| format!("##serie{i}"), latex_to_plain);
            let color = to_color32(p.color);
            // El nombre agrupa todo en una entrada de la leyenda; el id (único)
            // distingue cada elemento para egui_plot.
            let bar = |id: (usize, usize, u8), a: [f64; 2], b: [f64; 2]| {
                Line::new(name.clone(), PlotPoints::from(vec![a, b])).id(("barra", id)).color(color).width(1.2).allow_hover(false)
            };
            // Topes perpendiculares en cada extremo, de tamaño fijo en pantalla:
            // se pasa de píxeles a unidades del gráfico con la escala actual.
            let [dvx, dvy] = pui.transform().dvalue_dpos();
            let cap_px = if p.error_cap_pt > 0.0 { (p.error_cap_pt * PX_PER_PT * 1.6).max(3.0) as f64 } else { 0.0 };
            let (cx, cy) = (cap_px * dvx.abs(), cap_px * dvy.abs());
            for k in 0..p.x.len() {
                let (x, y) = (p.x[k], p.y[k]);
                if let Some(e) = &p.y_err
                    && e[k] > 0.0
                {
                    let (lo, hi) = (ty(y - e[k]), ty(y + e[k]));
                    pui.line(bar((i, k, 0), [tx(x), lo], [tx(x), hi]));
                    if cap_px > 0.0 {
                        pui.line(bar((i, k, 2), [tx(x) - cx, lo], [tx(x) + cx, lo]));
                        pui.line(bar((i, k, 3), [tx(x) - cx, hi], [tx(x) + cx, hi]));
                    }
                }
                if let Some(e) = &p.x_err
                    && e[k] > 0.0
                {
                    let (lo, hi) = (tx(x - e[k]), tx(x + e[k]));
                    pui.line(bar((i, k, 1), [lo, ty(y)], [hi, ty(y)]));
                    if cap_px > 0.0 {
                        pui.line(bar((i, k, 4), [lo, ty(y) - cy], [lo, ty(y) + cy]));
                        pui.line(bar((i, k, 5), [hi, ty(y) - cy], [hi, ty(y) + cy]));
                    }
                }
            }
            if let Some(connect) = &p.connect {
                let pts: Vec<[f64; 2]> = p.x.iter().zip(&p.y).map(|(x, y)| [tx(*x), ty(*y)]).collect();
                pui.line(
                    Line::new(name.clone(), PlotPoints::from(pts))
                        .id(("union", i))
                        .color(connect.color.map_or(color, to_color32))
                        .width(connect.width_pt * PX_PER_PT)
                        .style(dash_style(connect.dash)),
                );
            }
            if let Some(s) = shape(p.marker) {
                let pts: Vec<[f64; 2]> = p.x.iter().zip(&p.y).map(|(x, y)| [tx(*x), ty(*y)]).collect();
                pui.points(
                    Points::new(name, PlotPoints::from(pts))
                        .id(("puntos", i))
                        .shape(s)
                        .radius((p.marker_size_pt * 2.0).max(3.5))
                        .color(color)
                        .filled(true),
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_at_non_finite_points() {
        let pts = [[0.0, 1.0], [1.0, f64::NAN], [2.0, 2.0], [3.0, 3.0], [4.0, f64::INFINITY]];
        let s = segments(pts.into_iter());
        assert_eq!(s, vec![vec![[0.0, 1.0]], vec![[2.0, 2.0], [3.0, 3.0]]]);
    }
}
