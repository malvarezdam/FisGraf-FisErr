//! Opciones generales de la figura: tamaño, fuente, cuadrícula, leyenda y motor LaTeX.

use serde::{Deserialize, Serialize};

use super::style::Rgb;

/// Proporción ancho:alto del área del gráfico.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum AspectRatio {
    #[default]
    R4x3,
    R3x2,
    R16x9,
    R9x16,
    R1x1,
    /// Razón áurea (≈ 1,618:1).
    Golden,
    Custom {
        width: f64,
        height: f64,
    },
}

impl AspectRatio {
    /// Alto dividido por ancho.
    pub fn height_over_width(self) -> f64 {
        match self {
            Self::R4x3 => 3.0 / 4.0,
            Self::R3x2 => 2.0 / 3.0,
            Self::R16x9 => 9.0 / 16.0,
            Self::R9x16 => 16.0 / 9.0,
            Self::R1x1 => 1.0,
            Self::Golden => 2.0 / (1.0 + 5f64.sqrt()),
            Self::Custom { width, height } => height / width,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum FontChoice {
    /// Computer Modern / Latin Modern, la fuente clásica de LaTeX.
    #[default]
    LatinModern,
    /// Times (newtx).
    Times,
    /// Palatino (newpx).
    Palatino,
    Libertine,
    Charter,
    FiraSans,
    /// Cualquier fuente instalada en el sistema (solo con LuaLaTeX).
    System(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LatexEngine {
    #[default]
    LuaLatex,
    PdfLatex,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GridConfig {
    pub major: bool,
    pub minor: bool,
    pub color: Rgb,
    pub opacity: f32,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            major: true,
            minor: false,
            color: Rgb::GRAY,
            opacity: 0.3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum LegendPosition {
    NorthEast,
    NorthWest,
    #[default]
    SouthEast,
    SouthWest,
    /// Fuera del gráfico, a la derecha.
    OuterEast,
    /// Fuera del gráfico, debajo.
    Below,
    /// Coordenadas relativas al eje (0..1) de la esquina de anclaje.
    Custom { x: f64, y: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LegendConfig {
    pub show: bool,
    pub position: LegendPosition,
    pub columns: u32,
    pub frame: bool,
    /// Opacidad del fondo del cuadro.
    pub fill_opacity: f32,
}

impl Default for LegendConfig {
    fn default() -> Self {
        Self {
            show: true,
            position: LegendPosition::SouthEast,
            columns: 1,
            frame: true,
            fill_opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ResultsBoxConfig {
    pub show: bool,
    pub position: LegendPosition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FigureConfig {
    /// Título del gráfico en LaTeX. Vacío = sin título.
    pub title: String,
    /// Ancho del área del gráfico en centímetros.
    pub width_cm: f64,
    pub aspect: AspectRatio,
    pub font: FontChoice,
    pub font_size_pt: f32,
    pub grid: GridConfig,
    pub legend: LegendConfig,
    pub results_box: ResultsBoxConfig,
    /// Añade debajo un panel con los residuos de los ajustes.
    pub residuals_panel: bool,
    pub engine: LatexEngine,
    /// LaTeX adicional para el preámbulo (paquetes, macros).
    pub extra_preamble: String,
}

impl Default for FigureConfig {
    fn default() -> Self {
        Self {
            title: String::new(),
            width_cm: 10.0,
            aspect: AspectRatio::R4x3,
            font: FontChoice::LatinModern,
            font_size_pt: 10.0,
            grid: GridConfig::default(),
            legend: LegendConfig::default(),
            results_box: ResultsBoxConfig::default(),
            residuals_panel: false,
            engine: LatexEngine::LuaLatex,
            extra_preamble: String::new(),
        }
    }
}
