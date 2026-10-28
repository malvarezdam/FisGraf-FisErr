//! Genera el documento LaTeX (pgfplots) a partir de la escena y la configuración.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use crate::config::{
    AxisConfig, AxisScale, DashPattern, FontChoice, LatexEngine, LegendPosition, LineStyle, Marker, Notation,
    ProjectConfig, Rgb, TickDirection, TickSpacing,
};

use super::scene::{Scene, ScenePoints};

/// Documento generado.
#[derive(Debug, Clone, PartialEq)]
pub struct TexDocument {
    /// Documento `standalone` completo y compilable.
    pub full: String,
    /// Solo el entorno `tikzpicture` (para `\input{}` en un informe).
    pub picture: String,
    /// Paquetes que el fragmento necesita en el preámbulo del informe.
    pub preamble: String,
    pub warnings: Vec<String>,
}

pub fn generate(project: &ProjectConfig, scene: &Scene) -> TexDocument {
    let mut warnings = Vec::new();
    let preamble = preamble(project, scene, &mut warnings);
    let picture = picture(project, scene);
    let full = format!(
        "% Generado por FisGraf\n\\documentclass[border=4pt]{{standalone}}\n{preamble}\\begin{{document}}\n{picture}\\end{{document}}\n"
    );
    TexDocument { full, picture, preamble, warnings }
}

fn preamble(project: &ProjectConfig, scene: &Scene, warnings: &mut Vec<String>) -> String {
    let fig = &project.figure;
    let lua = fig.engine == LatexEngine::LuaLatex;
    let mut p = String::new();
    if !lua {
        p.push_str("\\usepackage[T1]{fontenc}\n");
    }
    p.push_str("\\usepackage{amsmath}\n");
    let font = match &fig.font {
        FontChoice::LatinModern if lua => String::new(),
        FontChoice::LatinModern => "\\usepackage{lmodern}\n".into(),
        FontChoice::Times => "\\usepackage{newtxtext,newtxmath}\n".into(),
        FontChoice::Palatino => "\\usepackage{newpxtext,newpxmath}\n".into(),
        FontChoice::Libertine => "\\usepackage{libertine}\n\\usepackage[libertine]{newtxmath}\n".into(),
        FontChoice::Charter => "\\usepackage{XCharter}\n\\usepackage[xcharter]{newtxmath}\n".into(),
        FontChoice::FiraSans => "\\usepackage[sfdefault]{FiraSans}\n\\usepackage{newtxsf}\n".into(),
        FontChoice::System(name) if lua => format!("\\usepackage{{fontspec}}\n\\setmainfont{{{name}}}\n"),
        FontChoice::System(name) => {
            warnings.push(format!("la fuente del sistema «{name}» requiere LuaLaTeX: se usa Latin Modern"));
            "\\usepackage{lmodern}\n".into()
        }
    };
    p.push_str(&font);
    p.push_str("\\usepackage{siunitx}\n");
    if project.y_axis.number_format.decimal_comma {
        p.push_str("\\sisetup{output-decimal-marker={,}}\n");
    }
    p.push_str("\\usepackage{pgfplots}\n\\pgfplotsset{compat=1.18}\n");
    if scene.curves.iter().any(|c| c.band.is_some()) {
        p.push_str("\\usepgfplotslibrary{fillbetween}\n");
    }
    if !fig.extra_preamble.trim().is_empty() {
        p.push_str(fig.extra_preamble.trim_end());
        p.push('\n');
    }
    p
}

/// Número para coordenadas: notación científica exacta, `nan` si no es finito.
fn num(v: f64) -> String {
    if v.is_finite() { format!("{v:e}") } else { "nan".into() }
}

/// Número para opciones (límites, distancias): forma decimal corta.
fn opt_num(v: f64) -> String {
    format!("{v}")
}

fn color_name(c: Rgb) -> String {
    format!("fg{}", &c.to_hex()[1..])
}

fn dash(d: DashPattern) -> &'static str {
    match d {
        DashPattern::Solid => "solid",
        DashPattern::Dashed => "dashed",
        DashPattern::DenselyDashed => "densely dashed",
        DashPattern::LooselyDashed => "loosely dashed",
        DashPattern::Dotted => "dotted",
        DashPattern::DenselyDotted => "densely dotted",
        DashPattern::DashDot => "dashdotted",
    }
}

fn mark(m: Marker) -> &'static str {
    match m {
        Marker::Circle => "*",
        Marker::Square => "square*",
        Marker::Triangle => "triangle*",
        Marker::Diamond => "diamond*",
        Marker::Cross => "x",
        Marker::Plus => "+",
        Marker::Star => "star",
        Marker::None => "none",
    }
}

fn line_opts(line: &LineStyle, color: Rgb) -> String {
    let mut s = format!("color={}, line width={}pt, {}", color_name(color), line.width_pt, dash(line.dash));
    if line.opacity < 1.0 {
        let _ = write!(s, ", opacity={}", line.opacity);
    }
    s
}

/// Opciones de un eje (`x` o `y`) comunes al gráfico principal y al de residuos.
fn axis_opts(axis: &AxisConfig, letter: char, limits: (Option<f64>, Option<f64>), with_ticks: bool) -> Vec<String> {
    let mut o = Vec::new();
    if let Some(v) = limits.0 {
        o.push(format!("{letter}min={}", opt_num(v)));
    }
    if let Some(v) = limits.1 {
        o.push(format!("{letter}max={}", opt_num(v)));
    }
    let log = axis.scale == AxisScale::Log10;
    if log {
        o.push(format!("{letter}mode=log"));
    }
    if with_ticks {
        match &axis.ticks.spacing {
            TickSpacing::Auto => {}
            TickSpacing::Distance(d) if !log => o.push(format!("{letter}tick distance={}", opt_num(*d))),
            TickSpacing::Distance(_) => {}
            TickSpacing::Manual(v) => {
                let list: Vec<String> = v.iter().map(|x| opt_num(*x)).collect();
                o.push(format!("{letter}tick={{{}}}", list.join(",")));
            }
        }
        if axis.ticks.minor_per_major > 0 {
            o.push(format!("minor {letter} tick num={}", axis.ticks.minor_per_major));
        }
    }
    let align = match axis.ticks.direction {
        TickDirection::In => "inside",
        TickDirection::Out => "outside",
        TickDirection::Both => "center",
    };
    o.push(format!("{letter}tick align={align}"));

    let f = &axis.number_format;
    let mut nf = Vec::new();
    match f.notation {
        Notation::Auto => {}
        Notation::Fixed => {
            nf.push(format!("fixed, precision={}", f.decimals));
            if f.fixed_zerofill {
                nf.push("fixed zerofill".into());
            }
        }
        Notation::Scientific => {
            nf.push(format!("sci, precision={}", f.decimals));
            if f.fixed_zerofill {
                nf.push("sci zerofill".into());
            }
        }
    }
    if f.decimal_comma {
        nf.push("use comma".into());
    }
    nf.push(if f.thousands_separator { "1000 sep={\\,}".into() } else { "1000 sep={}".into() });
    // Se vuelve a /tikz al final: la etiqueta del factor ×10ⁿ hereda este estilo.
    o.push(format!("{letter} tick label style={{/pgf/number format/.cd, {}, /tikz/.cd}}", nf.join(", ")));
    if log && f.notation == Notation::Fixed {
        o.push("log ticks with fixed point".into());
    }
    if !log {
        o.push(format!("scaled {letter} ticks={}", f.common_factor));
    }
    o
}

/// Posición de un cuadro (leyenda o resultados): (coordenadas relativas, ancla).
fn box_anchor(pos: LegendPosition) -> ((f64, f64), &'static str) {
    match pos {
        LegendPosition::NorthEast => ((0.97, 0.97), "north east"),
        LegendPosition::NorthWest => ((0.03, 0.97), "north west"),
        LegendPosition::SouthEast => ((0.97, 0.03), "south east"),
        LegendPosition::SouthWest => ((0.03, 0.03), "south west"),
        LegendPosition::OuterEast => ((1.03, 1.0), "north west"),
        LegendPosition::Below => ((0.5, -0.03), "north"),
        LegendPosition::Custom { x, y } => {
            let anchor = match (x > 0.5, y > 0.5) {
                (true, true) => "north east",
                (false, true) => "north west",
                (true, false) => "south east",
                (false, false) => "south west",
            };
            ((x, y), anchor)
        }
    }
}

fn picture(project: &ProjectConfig, scene: &Scene) -> String {
    let fig = &project.figure;
    let width = fig.width_cm;
    let height = width * fig.aspect.height_over_width();
    let residuals = fig.residuals_panel && !scene.residuals.is_empty();
    let (xmin, xmax) = scene.x_range;

    let mut colors: BTreeSet<Rgb> = BTreeSet::new();
    colors.insert(fig.grid.color);
    colors.extend(scene.points.iter().map(|p| p.color));
    colors.extend(scene.curves.iter().map(|c| c.color));
    colors.extend(scene.residuals.iter().map(|r| r.color));

    let mut t = String::new();
    let fs = fig.font_size_pt;
    let _ = writeln!(t, "\\begin{{tikzpicture}}[font=\\fontsize{{{fs}}}{{{}}}\\selectfont]", fs * 1.2);
    for c in &colors {
        let _ = writeln!(t, "\\definecolor{{{}}}{{HTML}}{{{}}}", color_name(*c), &c.to_hex()[1..]);
    }

    // --- Eje principal ---
    let mut o = vec![
        "name=main".to_owned(),
        format!("width={width}cm"),
        format!("height={height}cm"),
        "scale only axis".into(),
        "unbounded coords=jump".into(),
        "axis background/.style={fill=white}".into(),
    ];
    o.extend(axis_opts(&project.x_axis, 'x', (Some(xmin), Some(xmax)), true));
    o.extend(axis_opts(&project.y_axis, 'y', (project.y_axis.min, project.y_axis.max), true));
    if !fig.title.trim().is_empty() {
        o.push(format!("title={{{}}}", fig.title));
    }
    if residuals {
        o.push("xticklabels={}".into());
    } else if !project.x_axis.label.trim().is_empty() {
        o.push(format!("xlabel={{{}}}", project.x_axis.label));
    }
    if !project.y_axis.label.trim().is_empty() {
        o.push(format!("ylabel={{{}}}", project.y_axis.label));
    }
    let grid = match (fig.grid.major, fig.grid.minor) {
        (true, true) => Some("both"),
        (true, false) => Some("major"),
        (false, true) => Some("minor"),
        (false, false) => None,
    };
    if let Some(g) = grid {
        let gc = color_name(fig.grid.color);
        o.push(format!("grid={g}"));
        o.push(format!("major grid style={{color={gc}, opacity={}}}", fig.grid.opacity));
        o.push(format!("minor grid style={{color={gc}, opacity={}}}", fig.grid.opacity * 0.5));
    }

    let lg = &fig.legend;
    let ((lx, ly), anchor) = box_anchor(lg.position);
    let mut legend_style = vec![
        format!("at={{({lx},{ly})}}"),
        format!("anchor={anchor}"),
        format!("fill=white, fill opacity={}, text opacity=1", lg.fill_opacity),
        if lg.frame { "draw=black".into() } else { "draw=none".into() },
    ];
    if lg.position == LegendPosition::Below {
        // Debajo del título del eje x.
        legend_style[0] = "at={(0.5,0)}".into();
        legend_style.push("yshift=-3.2em".into());
    }
    o.push(format!("legend style={{{}}}", legend_style.join(", ")));
    o.push("legend cell align=left".into());
    let columns = if lg.position == LegendPosition::Below && lg.columns <= 1 { -1 } else { lg.columns.max(1) as i64 };
    if columns != 1 {
        o.push(format!("legend columns={columns}"));
        o.push("legend style={/tikz/every even column/.append style={column sep=0.8em}}".into());
    }

    let _ = writeln!(t, "\\begin{{axis}}[\n  {}\n]", o.join(",\n  "));

    // Bandas de confianza (debajo de todo).
    for (i, c) in scene.curves.iter().enumerate() {
        if let Some(b) = &c.band {
            let coords = |sel: fn(&(f64, f64, f64)) -> f64| {
                b.points.iter().map(|p| format!("({},{})", num(p.0), num(sel(p)))).collect::<Vec<_>>().join(" ")
            };
            let cn = color_name(c.color);
            let _ = writeln!(t, "\\addplot[name path=band{i}lo, draw=none, forget plot] coordinates {{{}}};", coords(|p| p.1));
            let _ = writeln!(t, "\\addplot[name path=band{i}hi, draw=none, forget plot] coordinates {{{}}};", coords(|p| p.2));
            let _ = writeln!(
                t,
                "\\addplot[fill={cn}, fill opacity={}, draw=none, forget plot] fill between[of=band{i}lo and band{i}hi];",
                b.opacity
            );
        }
    }

    for c in &scene.curves {
        let coords: Vec<String> = c.points.iter().map(|(x, y)| format!("({},{})", num(*x), num(*y))).collect();
        let forget = if c.legend.is_some() { "" } else { ", forget plot" };
        let _ = writeln!(t, "\\addplot[{}, no marks{forget}] coordinates {{{}}};", line_opts(&c.line, c.color), coords.join(" "));
        if let Some(l) = &c.legend {
            let _ = writeln!(t, "\\addlegendentry{{{l}}}");
        }
    }

    for p in &scene.points {
        write_points(&mut t, p);
    }

    if fig.results_box.show && !scene.results_box.is_empty() {
        let ((bx, by), anchor) = box_anchor(fig.results_box.position);
        let _ = writeln!(
            t,
            "\\node[draw=black, fill=white, anchor={anchor}, align=left, inner sep=3pt] at (rel axis cs:{bx},{by}) {{{}}};",
            scene.results_box.join("\\\\ ")
        );
    }
    t.push_str("\\end{axis}\n");

    // --- Panel de residuos ---
    if residuals {
        let mut o = vec![
            "at={(main.below south west)}".to_owned(),
            "anchor=north west".into(),
            "yshift=-0.25cm".into(),
            format!("width={width}cm"),
            format!("height={}cm", height * 0.3),
            "scale only axis".into(),
        ];
        o.extend(axis_opts(&project.x_axis, 'x', (Some(xmin), Some(xmax)), true));
        // Los residuos son mucho menores que y: formato automático con factor común ×10ⁿ.
        let mut y_res = project.y_axis.clone();
        y_res.scale = AxisScale::Linear;
        y_res.ticks.spacing = TickSpacing::Auto;
        y_res.number_format.notation = Notation::Auto;
        y_res.number_format.common_factor = true;
        o.extend(axis_opts(&y_res, 'y', (None, None), true));
        if !project.x_axis.label.trim().is_empty() {
            o.push(format!("xlabel={{{}}}", project.x_axis.label));
        }
        o.push("ylabel={Residuos}".into());
        if fig.grid.major {
            o.push("ymajorgrids".into());
            o.push(format!("major grid style={{color={}, opacity={}}}", color_name(fig.grid.color), fig.grid.opacity));
        }
        let _ = writeln!(t, "\\begin{{axis}}[\n  {}\n]", o.join(",\n  "));
        let _ = writeln!(t, "\\addplot[black, forget plot] coordinates {{({},0) ({},0)}};", num(xmin), num(xmax));
        for r in &scene.residuals {
            let p = ScenePoints {
                legend: None,
                color: r.color,
                marker: r.marker,
                marker_size_pt: r.marker_size_pt,
                error_cap_pt: r.error_cap_pt,
                connect: None,
                x: r.x.clone(),
                y: r.residuals.clone(),
                x_err: None,
                y_err: r.y_err.clone(),
            };
            write_points(&mut t, &p);
        }
        t.push_str("\\end{axis}\n");
    }
    t.push_str("\\end{tikzpicture}\n");
    t
}

fn write_points(t: &mut String, p: &ScenePoints) {
    let cn = color_name(p.color);
    let mut o = vec![format!("color={cn}"), format!("mark={}", mark(p.marker))];
    o.push(format!("mark size={}pt", p.marker_size_pt));
    o.push(format!("mark options={{solid, fill={cn}}}"));
    match &p.connect {
        Some(line) => o.push(line_opts(line, line.color.unwrap_or(p.color))),
        None => o.push("only marks".into()),
    }
    if p.legend.is_none() {
        o.push("forget plot".into());
    }
    let (hx, hy) = (p.x_err.is_some(), p.y_err.is_some());
    if hx || hy {
        let mut e = vec!["error bars/.cd".to_owned()];
        if hx {
            e.push("x dir=both, x explicit".into());
        }
        if hy {
            e.push("y dir=both, y explicit".into());
        }
        e.push(format!("error bar style={{color={cn}, line width=0.6pt, solid}}"));
        if p.error_cap_pt > 0.0 {
            e.push(format!("error mark options={{rotate=90, mark size={}pt, color={cn}, line width=0.6pt}}", p.error_cap_pt));
        } else {
            e.push("error mark=none".into());
        }
        o.push(e.join(", "));
    }
    let coords: Vec<String> = (0..p.x.len())
        .map(|i| {
            let base = format!("({},{})", num(p.x[i]), num(p.y[i]));
            if hx || hy {
                let ex = p.x_err.as_ref().map_or(0.0, |e| e[i]);
                let ey = p.y_err.as_ref().map_or(0.0, |e| e[i]);
                format!("{base} +- ({},{})", num(ex), num(ey))
            } else {
                base
            }
        })
        .collect();
    let _ = writeln!(t, "\\addplot[{}] coordinates {{{}}};", o.join(", "), coords.join(" "));
    if let Some(l) = &p.legend {
        let _ = writeln!(t, "\\addlegendentry{{{l}}}");
    }
}
