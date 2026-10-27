//! Tipos de estilo compartidos: colores, marcadores y líneas.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Color RGB. Se guarda en JSON como `"#RRGGBB"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// Paleta categórica por defecto (tab10 de Matplotlib).
    pub const PALETTE: [Rgb; 10] = [
        Rgb(0x1f, 0x77, 0xb4),
        Rgb(0xd6, 0x27, 0x28),
        Rgb(0x2c, 0xa0, 0x2c),
        Rgb(0xff, 0x7f, 0x0e),
        Rgb(0x94, 0x67, 0xbd),
        Rgb(0x8c, 0x56, 0x4b),
        Rgb(0xe3, 0x77, 0xc2),
        Rgb(0x7f, 0x7f, 0x7f),
        Rgb(0xbc, 0xbd, 0x22),
        Rgb(0x17, 0xbe, 0xcf),
    ];

    pub const BLACK: Rgb = Rgb(0, 0, 0);
    pub const GRAY: Rgb = Rgb(0x7f, 0x7f, 0x7f);

    /// Color `i` de la paleta, cíclico.
    pub fn palette(i: usize) -> Rgb {
        Self::PALETTE[i % Self::PALETTE.len()]
    }

    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("color inválido `{0}`: se espera el formato #RRGGBB")]
pub struct ParseRgbError(String);

impl FromStr for Rgb {
    type Err = ParseRgbError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParseRgbError(s.to_owned());
        let hex = s.trim().strip_prefix('#').ok_or_else(err)?;
        if hex.len() != 6 || !hex.is_ascii() {
            return Err(err());
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| err());
        Ok(Rgb(byte(0)?, byte(2)?, byte(4)?))
    }
}

impl TryFrom<String> for Rgb {
    type Error = ParseRgbError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<Rgb> for String {
    fn from(c: Rgb) -> Self {
        c.to_hex()
    }
}

/// Forma del marcador de los puntos experimentales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Marker {
    #[default]
    Circle,
    Square,
    Triangle,
    Diamond,
    Cross,
    Plus,
    Star,
    None,
}

/// Patrón de trazo de una línea.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DashPattern {
    #[default]
    Solid,
    Dashed,
    DenselyDashed,
    LooselyDashed,
    Dotted,
    DenselyDotted,
    DashDot,
}

/// Estilo de una línea (tendencia, función de referencia, unión de puntos).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LineStyle {
    /// `None` hereda el color de la serie asociada.
    pub color: Option<Rgb>,
    /// Grosor en puntos tipográficos.
    pub width_pt: f32,
    pub dash: DashPattern,
    /// Opacidad entre 0 y 1.
    pub opacity: f32,
}

impl Default for LineStyle {
    fn default() -> Self {
        Self {
            color: None,
            width_pt: 1.2,
            dash: DashPattern::Solid,
            opacity: 1.0,
        }
    }
}
