//! Líneas de tendencia: modelo teórico, método de ajuste y de incertidumbre.

use serde::{Deserialize, Serialize};

use super::style::LineStyle;

/// Parámetro libre del modelo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ParameterConfig {
    /// Nombre en la expresión, p. ej. `g`.
    pub name: String,
    /// Símbolo LaTeX opcional, p. ej. `g_{\mathrm{ef}}`. Vacío = se usa `name`.
    pub latex: String,
    /// Unidad en LaTeX (siunitx), p. ej. `\metre\per\second\squared`.
    pub unit: String,
    pub initial: f64,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    /// Si es `true`, el parámetro se mantiene en `initial` y no se ajusta.
    pub fixed: bool,
}

impl Default for ParameterConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            latex: String::new(),
            unit: String::new(),
            initial: 1.0,
            lower: None,
            upper: None,
            fixed: false,
        }
    }
}

/// Ecuación teórica escrita con sintaxis calculable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelConfig {
    /// Expresión, p. ej. `2*pi*sqrt((L + a)/g)`.
    pub expression: String,
    /// Nombre de la variable medida (independiente), p. ej. `L`.
    pub variable: String,
    pub parameters: Vec<ParameterConfig>,
    /// LaTeX escrito a mano para la ecuación. Vacío = conversión automática.
    pub latex_override: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            expression: "a*x + b".into(),
            variable: "x".into(),
            parameters: vec![
                ParameterConfig {
                    name: "a".into(),
                    ..Default::default()
                },
                ParameterConfig {
                    name: "b".into(),
                    initial: 0.0,
                    ..Default::default()
                },
            ],
            latex_override: String::new(),
        }
    }
}

/// Método numérico de ajuste.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FitMethod {
    /// Lineal si el modelo es lineal en los parámetros; si no, Levenberg–Marquardt.
    #[default]
    Auto,
    /// Mínimos cuadrados lineales (OLS, o WLS si hay pesos) por QR/SVD.
    Linear,
    LevenbergMarquardt,
    GaussNewton,
    NelderMead,
    /// Varianza efectiva: σ²_ef = σy² + (∂f/∂x · σx)².
    EffectiveVariance,
    /// Regresión de distancia ortogonal.
    Odr,
}

/// Criterios de parada del ajuste.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FitSettings {
    pub max_iterations: u32,
    /// Tolerancia relativa sobre el cambio de χ² y de los parámetros.
    pub tolerance: f64,
    /// Usa pesos 1/σ² cuando hay errores disponibles.
    pub use_weights: bool,
}

impl Default for FitSettings {
    fn default() -> Self {
        Self {
            max_iterations: 500,
            tolerance: 1e-10,
            use_weights: true,
        }
    }
}

/// Método para estimar la incertidumbre de los parámetros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UncertaintyMethod {
    #[default]
    Covariance,
    Bootstrap,
    MonteCarlo,
    Jackknife,
    /// Perfil de χ² con Δχ² = 1 (errores asimétricos).
    ChiSquareProfile,
}

/// Cómo se interpreta la matriz de covarianza.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CovarianceMode {
    /// Los σ de los datos son absolutos y conocidos.
    Absolute,
    /// La covarianza se escala por χ²_red (σ desconocidos o solo relativos).
    #[default]
    ScaledByReducedChi2,
}

/// Qué se re-muestrea en el bootstrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BootstrapKind {
    #[default]
    Residuals,
    Pairs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UncertaintyConfig {
    pub method: UncertaintyMethod,
    pub covariance_mode: CovarianceMode,
    pub bootstrap_kind: BootstrapKind,
    /// Número de re-muestreos (bootstrap y Monte Carlo).
    pub samples: u32,
    /// Nivel de confianza de intervalos y bandas, p. ej. 0.95.
    pub confidence_level: f64,
    /// Semilla para resultados reproducibles. `None` = aleatoria.
    pub seed: Option<u64>,
}

impl Default for UncertaintyConfig {
    fn default() -> Self {
        Self {
            method: UncertaintyMethod::Covariance,
            covariance_mode: CovarianceMode::ScaledByReducedChi2,
            bootstrap_kind: BootstrapKind::Residuals,
            samples: 2000,
            confidence_level: 0.95,
            seed: Some(42),
        }
    }
}

/// Una línea de tendencia. Se ajusta de forma independiente a cada serie de
/// `series_ids` usando el mismo modelo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FitConfig {
    pub id: u32,
    pub enabled: bool,
    pub series_ids: Vec<u32>,
    /// Texto de la leyenda en LaTeX. Vacío = la ecuación.
    pub label: String,
    pub show_in_legend: bool,
    pub model: ModelConfig,
    pub method: FitMethod,
    pub settings: FitSettings,
    pub uncertainty: UncertaintyConfig,
    /// El color `None` hereda el de la serie.
    pub line: LineStyle,
    /// Dibuja la banda de confianza del ajuste.
    pub show_band: bool,
    pub band_opacity: f32,
    /// Intervalo donde se dibuja la curva. `None` = rango de los datos.
    pub domain: Option<(f64, f64)>,
    pub samples: u32,
    /// Incluye los parámetros en el cuadro de resultados del gráfico.
    pub show_in_results_box: bool,
}

impl Default for FitConfig {
    fn default() -> Self {
        Self {
            id: 0,
            enabled: true,
            series_ids: Vec::new(),
            label: String::new(),
            show_in_legend: true,
            model: ModelConfig::default(),
            method: FitMethod::Auto,
            settings: FitSettings::default(),
            uncertainty: UncertaintyConfig::default(),
            line: LineStyle::default(),
            show_band: false,
            band_opacity: 0.2,
            domain: None,
            samples: 200,
            show_in_results_box: false,
        }
    }
}
