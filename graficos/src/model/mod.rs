//! Ecuaciones del usuario: parser a AST, evaluación, derivadas simbólicas
//! y conversión a LaTeX.
//!
//! La expresión se escribe con sintaxis calculable (`2*pi*sqrt((L + a)/g)`).
//! Un [`Model`] fija cuál nombre es la variable medida, cuáles son parámetros
//! por ajustar y cuáles constantes con valor conocido.

pub mod ast;
mod compile;
pub mod diff;
pub mod latex;
pub mod parser;
pub mod simplify;

use std::collections::BTreeSet;

pub use ast::{Expr, Func};
pub use latex::{LatexOptions, number_latex, symbol_latex, to_latex};
pub use parser::{ParseError, parse};

use compile::{Node, compile};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ModelError {
    #[error("error en la ecuación: {0}")]
    Parse(#[from] ParseError),

    #[error("nombres sin definir: {}. Decláralos como parámetro o como constante", .0.join(", "))]
    Unknown(Vec<String>),

    #[error("el parámetro «{0}» no aparece en la ecuación")]
    UnusedParameter(String),

    #[error("el nombre «{0}» está repetido")]
    Duplicate(String),

    #[error("«{0}» es un nombre reservado (función o constante)")]
    Reserved(String),

    #[error("«{0}» no es un nombre válido: usa letras, dígitos y _, empezando por letra")]
    InvalidName(String),

    #[error("la constante «{0}» no tiene un valor finito")]
    NonFiniteConstant(String),
}

/// Ecuación lista para evaluar y derivar.
#[derive(Debug, Clone)]
pub struct Model {
    expr: Expr,
    variable: String,
    params: Vec<String>,
    constants: Vec<(String, f64)>,
    value: Node,
    /// ∂f/∂pₖ simbólicas.
    d_params: Vec<Expr>,
    d_params_c: Vec<Node>,
    /// ∂f/∂x simbólica.
    d_x: Expr,
    d_x_c: Node,
    linear: bool,
}

impl Model {
    /// Analiza y valida la ecuación.
    ///
    /// - `variable`: la variable independiente (medida).
    /// - `params`: los parámetros a ajustar, en el orden en que se pasarán sus valores.
    /// - `constants`: nombres con valor fijo.
    pub fn new(
        expression: &str,
        variable: &str,
        params: &[String],
        constants: &[(String, f64)],
    ) -> Result<Self, ModelError> {
        let mut seen = BTreeSet::new();
        let names = std::iter::once(variable)
            .chain(params.iter().map(String::as_str))
            .chain(constants.iter().map(|(n, _)| n.as_str()));
        for name in names {
            if !ast::is_identifier(name) {
                return Err(ModelError::InvalidName(name.to_owned()));
            }
            if ast::is_reserved(name) {
                return Err(ModelError::Reserved(name.to_owned()));
            }
            if !seen.insert(name) {
                return Err(ModelError::Duplicate(name.to_owned()));
            }
        }
        if let Some((n, _)) = constants.iter().find(|(_, v)| !v.is_finite()) {
            return Err(ModelError::NonFiniteConstant(n.clone()));
        }

        let expr = parse(expression)?;
        let used = expr.free_symbols();
        let unknown: Vec<String> = used.iter().filter(|n| !seen.contains(n.as_str())).cloned().collect();
        if !unknown.is_empty() {
            return Err(ModelError::Unknown(unknown));
        }
        if let Some(p) = params.iter().find(|p| !used.contains(*p)) {
            return Err(ModelError::UnusedParameter(p.clone()));
        }

        let build = |e: &Expr| compile(e, variable, params, constants);
        let d_params: Vec<Expr> = params.iter().map(|p| diff::diff(&expr, p)).collect();
        // Lineal en los parámetros ⇔ ninguna ∂f/∂pₖ depende de un parámetro.
        let linear = d_params.iter().all(|d| params.iter().all(|p| !d.contains(p)));
        let d_x = diff::diff(&expr, variable);

        Ok(Self {
            value: build(&expr),
            d_params_c: d_params.iter().map(build).collect(),
            d_x_c: build(&d_x),
            expr,
            variable: variable.to_owned(),
            params: params.to_vec(),
            constants: constants.to_vec(),
            d_params,
            d_x,
            linear,
        })
    }

    pub fn expr(&self) -> &Expr {
        &self.expr
    }

    pub fn variable(&self) -> &str {
        &self.variable
    }

    pub fn params(&self) -> &[String] {
        &self.params
    }

    pub fn constants(&self) -> &[(String, f64)] {
        &self.constants
    }

    /// f(x; p).
    pub fn eval(&self, x: f64, p: &[f64]) -> f64 {
        debug_assert_eq!(p.len(), self.params.len());
        self.value.eval(x, p)
    }

    /// Gradiente respecto de los parámetros: `out[k] = ∂f/∂pₖ (x; p)`.
    pub fn gradient(&self, x: f64, p: &[f64], out: &mut [f64]) {
        for (o, d) in out.iter_mut().zip(&self.d_params_c) {
            *o = d.eval(x, p);
        }
    }

    /// ∂f/∂x (x; p), usada por la varianza efectiva y ODR.
    pub fn dfdx(&self, x: f64, p: &[f64]) -> f64 {
        self.d_x_c.eval(x, p)
    }

    /// Derivada simbólica respecto del parámetro `k`.
    pub fn param_derivative(&self, k: usize) -> &Expr {
        &self.d_params[k]
    }

    pub fn x_derivative(&self) -> &Expr {
        &self.d_x
    }

    /// `true` si f es lineal en los parámetros: f = f₀(x) + Σ pₖ·gₖ(x).
    /// En ese caso el ajuste por mínimos cuadrados tiene solución exacta.
    pub fn is_linear(&self) -> bool {
        self.linear
    }

    pub fn to_latex(&self, opts: &LatexOptions) -> String {
        to_latex(&self.expr, opts)
    }
}

/// Nombres de una expresión que no son la variable ni constantes conocidas:
/// candidatos a parámetro para que la interfaz los proponga automáticamente.
pub fn suggest_parameters(expression: &str, variable: &str, constants: &[String]) -> Result<Vec<String>, ParseError> {
    Ok(parse(expression)?
        .free_symbols()
        .into_iter()
        .filter(|n| n != variable && !constants.contains(n))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn model(expr: &str, var: &str, params: &[&str]) -> Model {
        Model::new(expr, var, &names(params), &[]).unwrap()
    }

    type Reference = fn(f64, &[f64]) -> f64;

    #[test]
    fn evaluation_matches_rust() {
        let cases: [(&str, &[&str], Reference); 6] = [
            ("a*x + b", &["a", "b"], |x, p| p[0] * x + p[1]),
            ("2*pi*sqrt((x + a)/g)", &["a", "g"], |x, p| {
                2.0 * std::f64::consts::PI * ((x + p[0]) / p[1]).sqrt()
            }),
            ("A*exp(-x/tau)*cos(w*x + phi)", &["A", "tau", "w", "phi"], |x, p| {
                p[0] * (-x / p[1]).exp() * (p[2] * x + p[3]).cos()
            }),
            ("a*x^3 - b/x^2", &["a", "b"], |x, p| p[0] * x.powi(3) - p[1] / x.powi(2)),
            ("(-x)^3 + a", &["a"], |x, p| (-x).powi(3) + p[0]),
            ("a*x^b", &["a", "b"], |x, p| p[0] * x.powf(p[1])),
        ];
        let p = [1.3, 0.7, 2.1, 0.4];
        for (src, params, f) in cases {
            let m = model(src, "x", params);
            let p = &p[..params.len()];
            for x in [0.3, 1.0, 2.5] {
                let (a, b) = (m.eval(x, p), f(x, p));
                assert!((a - b).abs() <= 1e-12 * b.abs().max(1.0), "{src} en x={x}: {a} vs {b}");
            }
        }
    }

    /// Las derivadas simbólicas coinciden con diferencias finitas centrales.
    #[test]
    fn derivatives_match_finite_differences() {
        let cases: [(&str, &[&str]); 9] = [
            ("2*pi*sqrt((x + a)/g)", &["a", "g"]),
            ("A*exp(-x/tau)*cos(w*x + phi)", &["A", "tau", "w", "phi"]),
            ("a*x^b", &["a", "b"]),
            ("b^x*a", &["a", "b"]),
            ("x^(a*x)", &["a"]),
            ("ln(a*x) + log10(b*x) + abs(x - a)", &["a", "b"]),
            ("tan(a*x) + asin(x/(4*b)) + acos(x/(5*a)) + atan(b*x)", &["a", "b"]),
            ("sinh(a*x) + cosh(b*x) + tanh(a*b*x)", &["a", "b"]),
            ("a/(1 + (x/b)^2)", &["a", "b"]),
        ];
        let p0 = [1.3, 0.8, 2.1, 0.4];
        for (src, params) in cases {
            let m = model(src, "x", params);
            let p = &p0[..params.len()];
            let mut grad = vec![0.0; p.len()];
            for x in [0.4, 1.1, 1.7] {
                m.gradient(x, p, &mut grad);
                for k in 0..p.len() {
                    let h = 1e-6 * p[k].abs().max(1.0);
                    let (mut up, mut dn) = (p.to_vec(), p.to_vec());
                    up[k] += h;
                    dn[k] -= h;
                    let fd = (m.eval(x, &up) - m.eval(x, &dn)) / (2.0 * h);
                    assert!((grad[k] - fd).abs() < 1e-6 * fd.abs().max(1.0), "{src} ∂/∂{}: {} vs {fd}", params[k], grad[k]);
                }
                let h = 1e-6;
                let fd = (m.eval(x + h, p) - m.eval(x - h, p)) / (2.0 * h);
                assert!((m.dfdx(x, p) - fd).abs() < 1e-6 * fd.abs().max(1.0), "{src} ∂/∂x");
            }
        }
    }

    #[test]
    fn linearity() {
        let lin = |src: &str, params: &[&str]| model(src, "x", params).is_linear();
        assert!(lin("a*x + b", &["a", "b"]));
        assert!(lin("a*x^2 + b*sin(x) + c", &["a", "b", "c"]));
        assert!(lin("a/x + 3*x", &["a"]));
        assert!(!lin("a*exp(b*x)", &["a", "b"]));
        assert!(!lin("2*pi*sqrt((x + a)/g)", &["a", "g"]));
        assert!(!lin("x/a", &["a"]));
        assert!(!lin("a*b*x", &["a", "b"]));
    }

    #[test]
    fn constants_and_latex() {
        let m = Model::new(
            "k*2*pi*sqrt(L/g)",
            "L",
            &names(&["k"]),
            &[("g".into(), 9.81)],
        )
        .unwrap();
        let expected = 0.5 * 2.0 * std::f64::consts::PI * (2.0f64 / 9.81).sqrt();
        assert!((m.eval(2.0, &[0.5]) - expected).abs() < 1e-12);
        assert!(m.is_linear());
        assert_eq!(m.to_latex(&LatexOptions::default()), "k \\cdot 2 \\pi \\sqrt{\\frac{L}{g}}");
    }

    #[test]
    fn validation_errors() {
        let err = |e: &str, v: &str, p: &[&str], c: &[(&str, f64)]| {
            let c: Vec<(String, f64)> = c.iter().map(|(n, v)| (n.to_string(), *v)).collect();
            Model::new(e, v, &names(p), &c).unwrap_err()
        };
        assert_eq!(
            err("a*x + b + c", "x", &["a"], &[]),
            ModelError::Unknown(names(&["b", "c"]))
        );
        assert_eq!(err("a*x", "x", &["a", "b"], &[]), ModelError::UnusedParameter("b".into()));
        assert_eq!(err("a*x", "x", &["a", "a"], &[]), ModelError::Duplicate("a".into()));
        assert_eq!(err("a*x", "x", &["x"], &[]), ModelError::Duplicate("x".into()));
        assert_eq!(err("e*x", "x", &["e"], &[]), ModelError::Reserved("e".into()));
        assert_eq!(err("a*x", "x", &["2a"], &[]), ModelError::InvalidName("2a".into()));
        assert_eq!(err("a*x*g", "x", &["a"], &[("g", f64::NAN)]), ModelError::NonFiniteConstant("g".into()));
        assert!(matches!(err("a*x +", "x", &["a"], &[]), ModelError::Parse(_)));
        assert_eq!(
            err("a*x + b", "x", &["a"], &[]).to_string(),
            "nombres sin definir: b. Decláralos como parámetro o como constante"
        );
    }

    #[test]
    fn parameter_suggestions() {
        let s = suggest_parameters("A*exp(-x/tau) + y0", "x", &["y0".into()]).unwrap();
        assert_eq!(s, ["A", "tau"]);
    }
}
