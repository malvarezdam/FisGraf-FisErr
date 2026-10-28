//! Contenido ya calculado de un gráfico: puntos, curvas muestreadas, residuos
//! y líneas del cuadro de resultados. El generador LaTeX solo lee esto y la
//! configuración de ejes y figura.

use crate::config::{LineStyle, Marker, Rgb};

#[derive(Debug, Clone, PartialEq)]
pub struct ScenePoints {
    /// Texto LaTeX de la leyenda; `None` = no aparece en la leyenda.
    pub legend: Option<String>,
    pub color: Rgb,
    pub marker: Marker,
    pub marker_size_pt: f32,
    pub error_cap_pt: f32,
    pub connect: Option<LineStyle>,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub x_err: Option<Vec<f64>>,
    pub y_err: Option<Vec<f64>>,
}

/// Banda de confianza: (x, inferior, superior).
#[derive(Debug, Clone, PartialEq)]
pub struct SceneBand {
    pub points: Vec<(f64, f64, f64)>,
    pub opacity: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneCurve {
    pub legend: Option<String>,
    /// Estilo con el color ya resuelto.
    pub line: LineStyle,
    pub color: Rgb,
    /// Puntos muestreados; los no finitos cortan la curva.
    pub points: Vec<(f64, f64)>,
    pub band: Option<SceneBand>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneResiduals {
    pub color: Rgb,
    pub marker: Marker,
    pub marker_size_pt: f32,
    pub error_cap_pt: f32,
    pub x: Vec<f64>,
    pub residuals: Vec<f64>,
    pub y_err: Option<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scene {
    pub points: Vec<ScenePoints>,
    pub curves: Vec<SceneCurve>,
    pub residuals: Vec<SceneResiduals>,
    /// Líneas del cuadro de resultados, en LaTeX (modo texto).
    pub results_box: Vec<String>,
    /// Intervalo del eje x que se dibuja.
    pub x_range: (f64, f64),
    pub warnings: Vec<String>,
}
