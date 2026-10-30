//! Series de mediciones: de dónde salen los datos y cómo se dibujan.

use serde::{Deserialize, Serialize};

use super::style::{LineStyle, Marker, Rgb};

/// Incertidumbre asociada a una variable medida.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ErrorSpec {
    /// Sin barras de error.
    #[default]
    None,
    /// Rango de celdas con un error por medición, p. ej. `"Hoja1!C2:C20"`.
    Range(String),
    /// Mismo error absoluto para todas las mediciones.
    Constant(f64),
    /// Porcentaje del valor medido (5.0 = 5 %).
    Percent(f64),
    /// Resolución del instrumento; σ = resolución / (2√3) según la GUM
    /// (distribución uniforme).
    Resolution(f64),
}

/// Una serie de mediciones dentro del gráfico.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SeriesConfig {
    /// Identificador estable; los ajustes se refieren a la serie por él.
    pub id: u32,
    /// Texto de la leyenda, en LaTeX.
    pub label: String,
    pub show_in_legend: bool,
    pub visible: bool,
    /// Rango de la variable independiente, p. ej. `"Hoja1!A2:A20"`.
    pub x_range: String,
    /// Rango de la variable dependiente, p. ej. `"Hoja1!B2:B20"`.
    pub y_range: String,
    pub x_error: ErrorSpec,
    pub y_error: ErrorSpec,
    pub color: Rgb,
    pub marker: Marker,
    /// Tamaño del marcador en puntos tipográficos.
    pub marker_size_pt: f32,
    /// Une los puntos con una línea (normalmente desactivado en física).
    pub connect_points: Option<LineStyle>,
    /// Dibuja las barras de error (si las hay). Si es falso solo se ven los
    /// puntos; los errores se siguen usando en los ajustes.
    pub error_bars: bool,
    /// Ancho de los topes de las barras de error en puntos (0 = sin tope).
    pub error_cap_pt: f32,
}

impl Default for SeriesConfig {
    fn default() -> Self {
        Self {
            id: 0,
            label: "Datos".into(),
            show_in_legend: true,
            visible: true,
            x_range: String::new(),
            y_range: String::new(),
            x_error: ErrorSpec::None,
            y_error: ErrorSpec::None,
            color: Rgb::palette(0),
            marker: Marker::Circle,
            marker_size_pt: 2.0,
            connect_points: None,
            error_bars: true,
            error_cap_pt: 2.0,
        }
    }
}
