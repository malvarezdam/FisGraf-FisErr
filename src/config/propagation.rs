//! Calculadora de propagación de errores: una ecuación, sus variables medidas
//! (con incertidumbre) y sus constantes.

use serde::{Deserialize, Serialize};

/// Método para propagar las incertidumbres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PropagationMethod {
    /// Primer orden (GUM): σ_f² = Σ (∂f/∂xᵢ)² σᵢ² + covarianzas.
    #[default]
    Linear,
    /// Segundo orden (GUM §5.1.2): añade derivadas segundas y terceras.
    SecondOrder,
    /// Monte Carlo (GUM, Suplemento 1).
    MonteCarlo,
    /// Kragten: diferencias finitas con desplazamientos de ±σ.
    Kragten,
    /// Cota máxima: Σ |∂f/∂xᵢ| σᵢ (conservadora).
    MaxBound,
}

/// Distribución de una magnitud de entrada (para Monte Carlo). En todos los
/// casos `sigma` es la incertidumbre estándar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum InputDistribution {
    #[default]
    Normal,
    /// Rectangular de semiancho a = √3·σ (p. ej. resolución de un instrumento).
    Uniform,
    /// Triangular simétrica de semiancho a = √6·σ.
    Triangular,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PropagationVariable {
    pub name: String,
    pub value: f64,
    /// Incertidumbre estándar. Se ignora si `constant`.
    pub sigma: f64,
    /// `true` = constante exacta (sin error).
    pub constant: bool,
    pub distribution: InputDistribution,
    /// Símbolo LaTeX opcional. Vacío = automático.
    pub latex: String,
    /// Unidad (siunitx), solo informativa.
    pub unit: String,
}

impl Default for PropagationVariable {
    fn default() -> Self {
        Self {
            name: String::new(),
            value: 1.0,
            sigma: 0.0,
            constant: false,
            distribution: InputDistribution::Normal,
            latex: String::new(),
            unit: String::new(),
        }
    }
}

/// Coeficiente de correlación entre dos variables medidas.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Correlation {
    pub a: String,
    pub b: String,
    pub rho: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PropagationConfig {
    pub id: u32,
    /// Nombre de la magnitud calculada, p. ej. `g`.
    pub name: String,
    /// Símbolo LaTeX del resultado. Vacío = a partir de `name`.
    pub latex: String,
    /// Unidad del resultado (siunitx).
    pub unit: String,
    pub expression: String,
    pub variables: Vec<PropagationVariable>,
    pub correlations: Vec<Correlation>,
    pub method: PropagationMethod,
    /// Muestras de Monte Carlo.
    pub samples: u32,
    pub seed: Option<u64>,
    /// Nivel de confianza de la incertidumbre expandida, p. ej. 0.95.
    pub confidence_level: f64,
}

impl Default for PropagationConfig {
    fn default() -> Self {
        Self {
            id: 0,
            name: "f".into(),
            latex: String::new(),
            unit: String::new(),
            expression: String::new(),
            variables: Vec::new(),
            correlations: Vec::new(),
            method: PropagationMethod::Linear,
            samples: 200_000,
            seed: Some(42),
            confidence_level: 0.95,
        }
    }
}
