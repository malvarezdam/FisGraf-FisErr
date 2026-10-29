//! Configuración de cada eje: título, límites, escala, marcas y formato numérico.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AxisScale {
    #[default]
    Linear,
    Log10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Notation {
    /// pgfplots elige.
    #[default]
    Auto,
    /// Notación decimal: 0,95.
    Fixed,
    /// Notación científica: 9,5·10⁻¹.
    Scientific,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NumberFormat {
    pub notation: Notation,
    pub decimals: u8,
    /// Muestra siempre todos los decimales (0,90 en lugar de 0,9).
    pub fixed_zerofill: bool,
    /// Usa coma decimal en lugar de punto.
    pub decimal_comma: bool,
    /// Separa los miles (1 000).
    pub thousands_separator: bool,
    /// Permite que pgfplots saque un factor común ×10ⁿ al extremo del eje.
    pub common_factor: bool,
}

impl Default for NumberFormat {
    fn default() -> Self {
        Self {
            notation: Notation::Auto,
            decimals: 2,
            fixed_zerofill: true,
            decimal_comma: true,
            thousands_separator: false,
            common_factor: false,
        }
    }
}

/// Separación entre marcas principales.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum TickSpacing {
    #[default]
    Auto,
    /// Distancia fija entre marcas.
    Distance(f64),
    /// Posiciones exactas.
    Manual(Vec<f64>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TickDirection {
    #[default]
    In,
    Out,
    Both,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TickConfig {
    pub spacing: TickSpacing,
    /// Marcas menores entre dos principales (0 = ninguna).
    pub minor_per_major: u32,
    pub direction: TickDirection,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AxisConfig {
    /// Título del eje en LaTeX, p. ej. `Periodo $T$ [\si{\second}]`.
    pub label: String,
    /// Límites manuales. `None` = automáticos.
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub scale: AxisScale,
    pub ticks: TickConfig,
    pub number_format: NumberFormat,
}
