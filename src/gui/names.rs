//! Nombres y descripciones en español de las opciones de configuración.

use crate::config::{
    AxisScale, BootstrapKind, CovarianceMode, FitMethod, LatexEngine, Notation, TickDirection, UncertaintyMethod,
};

pub const FIT_METHODS: [(FitMethod, &str); 7] = [
    (FitMethod::Auto, "Automático"),
    (FitMethod::Linear, "Mínimos cuadrados lineales"),
    (FitMethod::LevenbergMarquardt, "Levenberg–Marquardt"),
    (FitMethod::GaussNewton, "Gauss–Newton"),
    (FitMethod::NelderMead, "Nelder–Mead + LM"),
    (FitMethod::EffectiveVariance, "Varianza efectiva"),
    (FitMethod::Odr, "ODR (distancia ortogonal)"),
];

pub fn fit_method_help(m: FitMethod) -> &'static str {
    match m {
        FitMethod::Auto => "Varianza efectiva si hay errores en x; lineal si el modelo es lineal en los parámetros; si no, Levenberg–Marquardt.",
        FitMethod::Linear => "Solución exacta por SVD. Solo para modelos lineales en los parámetros (rectas, polinomios…).",
        FitMethod::LevenbergMarquardt => "El estándar para modelos no lineales (como scipy.curve_fit o MINPACK).",
        FitMethod::GaussNewton => "Rápido con buenos valores iniciales y problemas bien condicionados.",
        FitMethod::NelderMead => "Sin derivadas: útil si los valores iniciales están lejos. Se refina con LM al final.",
        FitMethod::EffectiveVariance => "Considera errores en x: σ²ef = σy² + (f′·σx)². Habitual en laboratorios de física.",
        FitMethod::Odr => "Tratamiento riguroso de errores en x e y (ODRPACK, scipy.odr).",
    }
}

pub const UNCERTAINTY_METHODS: [(UncertaintyMethod, &str); 5] = [
    (UncertaintyMethod::Covariance, "Matriz de covarianza"),
    (UncertaintyMethod::Bootstrap, "Bootstrap"),
    (UncertaintyMethod::MonteCarlo, "Monte Carlo"),
    (UncertaintyMethod::Jackknife, "Jackknife"),
    (UncertaintyMethod::ChiSquareProfile, "Perfil de χ² (asimétrico)"),
];

pub fn uncertainty_help(m: UncertaintyMethod) -> &'static str {
    match m {
        UncertaintyMethod::Covariance => "(JᵀWJ)⁻¹ en el óptimo. Rápido y estándar.",
        UncertaintyMethod::Bootstrap => "Re-muestrea los datos y ajusta muchas veces. No supone errores normales.",
        UncertaintyMethod::MonteCarlo => "Suma ruido con σx, σy a las mediciones y re-ajusta (GUM, Suplemento 1). Necesita errores.",
        UncertaintyMethod::Jackknife => "Deja fuera un punto cada vez: muestra cuánto pesa cada medición.",
        UncertaintyMethod::ChiSquareProfile => "Busca dónde χ² sube Δχ²: errores −/+ distintos en modelos muy no lineales.",
    }
}

pub const COVARIANCE_MODES: [(CovarianceMode, &str); 2] = [
    (CovarianceMode::ScaledByReducedChi2, "Escalada por χ²_red (σ desconocidas)"),
    (CovarianceMode::Absolute, "Absoluta (σ conocidas)"),
];

pub const BOOTSTRAP_KINDS: [(BootstrapKind, &str); 2] = [
    (BootstrapKind::Residuals, "Residuos"),
    (BootstrapKind::Pairs, "Pares (x, y)"),
];

pub const SCALES: [(AxisScale, &str); 2] = [(AxisScale::Linear, "Lineal"), (AxisScale::Log10, "Logarítmica")];

pub const NOTATIONS: [(Notation, &str); 3] = [
    (Notation::Auto, "Automática"),
    (Notation::Fixed, "Decimal (0,95)"),
    (Notation::Scientific, "Científica (9,5·10⁻¹)"),
];

pub const TICK_DIRECTIONS: [(TickDirection, &str); 3] = [
    (TickDirection::In, "Hacia dentro"),
    (TickDirection::Out, "Hacia fuera"),
    (TickDirection::Both, "A ambos lados"),
];

pub const ENGINES: [(LatexEngine, &str); 2] = [
    (LatexEngine::LuaLatex, "LuaLaTeX (cualquier fuente)"),
    (LatexEngine::PdfLatex, "pdfLaTeX"),
];

pub const PROPAGATION_METHODS: [(crate::config::PropagationMethod, &str); 5] = [
    (crate::config::PropagationMethod::Linear, "Lineal (derivadas parciales, GUM)"),
    (crate::config::PropagationMethod::SecondOrder, "Segundo orden (GUM §5.1.2)"),
    (crate::config::PropagationMethod::MonteCarlo, "Monte Carlo (GUM Suplemento 1)"),
    (crate::config::PropagationMethod::Kragten, "Kragten (diferencias finitas)"),
    (crate::config::PropagationMethod::MaxBound, "Cota máxima (Σ|∂f/∂x|·σ)"),
];

pub fn propagation_help(m: crate::config::PropagationMethod) -> &'static str {
    use crate::config::PropagationMethod as P;
    match m {
        P::Linear => "σ_f = √(Σ (∂f/∂xᵢ)² σᵢ²) + covarianzas. El método estándar; exacto si f es lineal en el rango de los errores.",
        P::SecondOrder => "Añade términos con derivadas segundas y terceras y corrige el valor esperado. Úsalo si f es muy curva o los errores relativos son grandes.",
        P::MonteCarlo => "Simula muchas mediciones posibles y mira cómo se distribuye f. No supone linealidad; da intervalos asimétricos si corresponde.",
        P::Kragten => "Mueve cada variable ±σ y mide el cambio de f. Equivale al lineal sin derivar; es el método de las guías EURACHEM.",
        P::MaxBound => "Suma de los errores en valor absoluto: una cota del peor caso, más grande que σ. Habitual en cursos introductorios; no es una incertidumbre estándar.",
    }
}

pub const DISTRIBUTIONS: [(crate::config::InputDistribution, &str); 3] = [
    (crate::config::InputDistribution::Normal, "Normal"),
    (crate::config::InputDistribution::Uniform, "Uniforme"),
    (crate::config::InputDistribution::Triangular, "Triangular"),
];
