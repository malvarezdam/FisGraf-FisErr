//! Funciones de referencia: curvas teóricas fijas para comparar con los datos.

use serde::{Deserialize, Serialize};

use super::style::{DashPattern, LineStyle, Rgb};

/// Constante con nombre usada dentro de una expresión.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NamedValue {
    pub name: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReferenceFunction {
    pub id: u32,
    pub enabled: bool,
    /// Texto de la leyenda en LaTeX. Vacío = la ecuación.
    pub label: String,
    pub show_in_legend: bool,
    /// Expresión calculable, p. ej. `0.987*2*pi*sqrt(x/9.81)`.
    pub expression: String,
    pub variable: String,
    pub constants: Vec<NamedValue>,
    /// LaTeX escrito a mano para la ecuación. Vacío = conversión automática.
    pub latex_override: String,
    /// Intervalo donde se dibuja. `None` = todo el eje x.
    pub domain: Option<(f64, f64)>,
    pub samples: u32,
    pub line: LineStyle,
}

impl Default for ReferenceFunction {
    fn default() -> Self {
        Self {
            id: 0,
            enabled: true,
            label: String::new(),
            show_in_legend: true,
            expression: String::new(),
            variable: "x".into(),
            constants: Vec::new(),
            latex_override: String::new(),
            domain: None,
            samples: 200,
            line: LineStyle {
                color: Some(Rgb::GRAY),
                dash: DashPattern::Dashed,
                ..Default::default()
            },
        }
    }
}
