//! Conversión de expresiones a LaTeX (modo matemático).

use std::collections::HashMap;

use super::ast::{Expr, Func};

#[derive(Debug, Clone, Default)]
pub struct LatexOptions {
    /// Escribe `0{,}96` en lugar de `0.96`.
    pub decimal_comma: bool,
    /// LaTeX elegido por el usuario para nombres concretos, p. ej. `g` → `g_{\mathrm{ef}}`.
    pub symbols: HashMap<String, String>,
}

// Precedencias al imprimir.
const ADD: u8 = 1;
const MUL: u8 = 2;
const NEG: u8 = 3;
const FRAC: u8 = 4;
const POW: u8 = 5;
const ATOM: u8 = 6;

/// LaTeX de una expresión (sin `$`).
pub fn to_latex(e: &Expr, opts: &LatexOptions) -> String {
    tex(e, opts).0
}

fn paren(s: &str) -> String {
    format!("\\left({s}\\right)")
}

/// Aplica paréntesis si la precedencia es menor que `min` o el texto empieza con signo.
fn wrap((s, p): (String, u8), min: u8) -> String {
    if p < min || s.starts_with('-') { paren(&s) } else { s }
}

fn tex(e: &Expr, o: &LatexOptions) -> (String, u8) {
    match e {
        Expr::Num(v) => {
            let s = number_latex(*v, o.decimal_comma);
            let p = if *v < 0.0 {
                NEG
            } else if s.contains("\\times") {
                MUL
            } else {
                ATOM
            };
            (s, p)
        }
        Expr::Pi => ("\\pi".into(), ATOM),
        Expr::E => ("e".into(), ATOM),
        Expr::Var(n) => (o.symbols.get(n).cloned().unwrap_or_else(|| symbol_latex(n)), ATOM),
        Expr::Neg(a) => {
            let inner = tex(a, o);
            let s = if inner.1 <= ADD || inner.0.starts_with('-') { paren(&inner.0) } else { inner.0 };
            (format!("-{s}"), NEG)
        }
        Expr::Add(a, b) => (format!("{} + {}", tex(a, o).0, wrap(tex(b, o), MUL)), ADD),
        Expr::Sub(a, b) => (format!("{} - {}", tex(a, o).0, wrap(tex(b, o), MUL)), ADD),
        Expr::Mul(a, b) => {
            let l = tex(a, o);
            let l = if l.1 < MUL { paren(&l.0) } else { l.0 };
            let r = tex(b, o);
            let r = if r.1 < MUL || r.1 == NEG { paren(&r.0) } else { r.0 };
            let sep = if r.starts_with(|c: char| c.is_ascii_digit()) { " \\cdot " } else { " " };
            (format!("{l}{sep}{r}"), MUL)
        }
        // -x/τ se lee (-x)/τ: el signo va delante de la fracción.
        Expr::Div(a, b) => match &**a {
            Expr::Neg(inner) => (format!("-\\frac{{{}}}{{{}}}", tex(inner, o).0, tex(b, o).0), NEG),
            _ => (format!("\\frac{{{}}}{{{}}}", tex(a, o).0, tex(b, o).0), FRAC),
        },
        Expr::Pow(a, b) => pow_latex(a, b, o),
        Expr::Call(f, a) => call_latex(*f, a, o),
    }
}

fn pow_latex(a: &Expr, b: &Expr, o: &LatexOptions) -> (String, u8) {
    let exp = tex(b, o).0;
    if let Expr::Num(n) = b {
        if *n == 0.5 {
            return (format!("\\sqrt{{{}}}", tex(a, o).0), ATOM);
        }
        // sin(x)^2 → \sin^{2}\left(x\right)
        if let Expr::Call(f, arg) = a
            && n.fract() == 0.0
            && *n > 0.0
            && let Some(name) = operator_name(*f)
        {
            return (format!("{name}^{{{exp}}}{}", paren(&tex(arg, o).0)), ATOM);
        }
    }
    let base = tex(a, o);
    let base = if base.1 < ATOM || base.0.starts_with('-') { paren(&base.0) } else { base.0 };
    (format!("{base}^{{{exp}}}"), POW)
}

/// Funciones que se escriben como operador seguido del argumento entre paréntesis.
fn operator_name(f: Func) -> Option<&'static str> {
    Some(match f {
        Func::Sin => "\\sin",
        Func::Cos => "\\cos",
        Func::Tan => "\\tan",
        Func::Asin => "\\arcsin",
        Func::Acos => "\\arccos",
        Func::Atan => "\\arctan",
        Func::Sinh => "\\sinh",
        Func::Cosh => "\\cosh",
        Func::Tanh => "\\tanh",
        Func::Ln => "\\ln",
        Func::Log10 => "\\log_{10}",
        Func::Sign => "\\operatorname{sgn}",
        Func::Exp | Func::Sqrt | Func::Abs => return None,
    })
}

fn call_latex(f: Func, a: &Expr, o: &LatexOptions) -> (String, u8) {
    let arg = tex(a, o).0;
    match f {
        Func::Sqrt => (format!("\\sqrt{{{arg}}}"), ATOM),
        Func::Abs => (format!("\\left|{arg}\\right|"), ATOM),
        // Un exponente largo se lee mal en superíndice: se usa \exp(…).
        Func::Exp if arg.len() > 24 => (format!("\\exp{}", paren(&arg)), ATOM),
        Func::Exp => (format!("e^{{{arg}}}"), POW),
        _ => {
            let name = operator_name(f).expect("todas las demás funciones tienen operador");
            (format!("{name}{}", paren(&arg)), ATOM)
        }
    }
}

/// Número en LaTeX: enteros tal cual, valores muy grandes o pequeños en
/// notación científica (`1{,}5 \times 10^{-7}`).
pub fn number_latex(v: f64, decimal_comma: bool) -> String {
    if v < 0.0 {
        return format!("-{}", number_latex(-v, decimal_comma));
    }
    let s = if v == 0.0 || (v.fract() == 0.0 && v < 1e6) {
        format!("{v:.0}")
    } else if !(1e-3..1e6).contains(&v) {
        let sci = format!("{v:e}");
        let (mantissa, exp) = sci.split_once('e').expect("formato {:e}");
        if mantissa == "1" {
            format!("10^{{{exp}}}")
        } else {
            format!("{mantissa} \\times 10^{{{exp}}}")
        }
    } else {
        format!("{v}")
    };
    if decimal_comma { s.replace('.', "{,}") } else { s }
}

const GREEK: [&str; 37] = [
    "alpha", "beta", "gamma", "delta", "epsilon", "varepsilon", "zeta", "eta", "theta", "vartheta", "iota",
    "kappa", "lambda", "mu", "nu", "xi", "rho", "varrho", "sigma", "tau", "upsilon", "phi", "varphi", "chi",
    "psi", "omega", "Gamma", "Delta", "Theta", "Lambda", "Xi", "Pi", "Sigma", "Upsilon", "Phi", "Psi", "Omega",
];

fn greek_latex(name: &str) -> Option<String> {
    if GREEK.contains(&name) {
        return Some(format!("\\{name}"));
    }
    const UNICODE: [(char, &str); 34] = [
        ('α', "alpha"),
        ('β', "beta"),
        ('γ', "gamma"),
        ('δ', "delta"),
        ('ε', "varepsilon"),
        ('ζ', "zeta"),
        ('η', "eta"),
        ('θ', "theta"),
        ('ι', "iota"),
        ('κ', "kappa"),
        ('λ', "lambda"),
        ('μ', "mu"),
        ('ν', "nu"),
        ('ξ', "xi"),
        ('π', "pi"),
        ('ρ', "rho"),
        ('σ', "sigma"),
        ('τ', "tau"),
        ('υ', "upsilon"),
        ('φ', "varphi"),
        ('χ', "chi"),
        ('ψ', "psi"),
        ('ω', "omega"),
        ('Γ', "Gamma"),
        ('Δ', "Delta"),
        ('Θ', "Theta"),
        ('Λ', "Lambda"),
        ('Ξ', "Xi"),
        ('Π', "Pi"),
        ('Σ', "Sigma"),
        ('Υ', "Upsilon"),
        ('Φ', "Phi"),
        ('Ψ', "Psi"),
        ('Ω', "Omega"),
    ];
    let mut chars = name.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return None;
    };
    UNICODE.iter().find(|(u, _)| *u == c).map(|(_, n)| format!("\\{n}"))
}

/// LaTeX por defecto de un nombre: `omega` → `\omega`, `T_teo` → `T_{\mathrm{teo}}`,
/// `x0` → `x_{0}`, `Tmax` → `\mathrm{Tmax}`.
pub fn symbol_latex(name: &str) -> String {
    let (base, sub) = match name.split_once('_') {
        Some((b, s)) if !b.is_empty() && !s.is_empty() => (b, Some(s)),
        _ => {
            // Dígitos finales como subíndice: x0, v12, omega0.
            let digits = name.trim_end_matches(|c: char| c.is_ascii_digit());
            if digits.len() < name.len() && !digits.is_empty() {
                (digits, Some(&name[digits.len()..]))
            } else {
                (name, None)
            }
        }
    };
    let part = |s: &str| {
        greek_latex(s).unwrap_or_else(|| {
            if s.chars().count() == 1 || s.chars().all(|c| c.is_ascii_digit()) {
                s.to_owned()
            } else {
                format!("\\mathrm{{{}}}", s.replace('_', "\\_"))
            }
        })
    };
    match sub {
        Some(s) => format!("{}_{{{}}}", part(base), part(s)),
        None => part(base),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::parser::parse;

    fn l(src: &str) -> String {
        to_latex(&parse(src).unwrap(), &LatexOptions::default())
    }

    #[test]
    fn physics_formulas() {
        assert_eq!(l("2*pi*sqrt((L + a)/g)"), "2 \\pi \\sqrt{\\frac{L + a}{g}}");
        assert_eq!(l("A*exp(-x/tau)"), "A e^{-\\frac{x}{\\tau}}");
        assert_eq!(l("exp(x/tau)"), "e^{\\frac{x}{\\tau}}");
        assert_eq!(
            l("exp(-(x - x0)^2/(2*sigma^2))"),
            "\\exp\\left(-\\frac{\\left(x - x_{0}\\right)^{2}}{2 \\sigma^{2}}\\right)"
        );
        assert_eq!(l("A*sin(omega*t + phi)^2"), "A \\sin^{2}\\left(\\omega t + \\phi\\right)");
        assert_eq!(l("1/2*m*v^2"), "\\frac{1}{2} m v^{2}");
        assert_eq!(l("(a + b)^2"), "\\left(a + b\\right)^{2}");
        assert_eq!(l("(a/b)^n"), "\\left(\\frac{a}{b}\\right)^{n}");
        assert_eq!(l("a - (b + c)"), "a - \\left(b + c\\right)");
        assert_eq!(l("a*(-b)"), "a \\left(-b\\right)");
        assert_eq!(l("-(a + b)"), "-\\left(a + b\\right)");
        assert_eq!(l("2*3"), "2 \\cdot 3");
        assert_eq!(l("x^0.5 + abs(y) + log10(z)"), "\\sqrt{x} + \\left|y\\right| + \\log_{10}\\left(z\\right)");
        assert_eq!(l("e^(-x)"), "e^{-x}");
    }

    #[test]
    fn symbols() {
        assert_eq!(symbol_latex("x"), "x");
        assert_eq!(symbol_latex("omega"), "\\omega");
        assert_eq!(symbol_latex("ω"), "\\omega");
        assert_eq!(symbol_latex("T_teo"), "T_{\\mathrm{teo}}");
        assert_eq!(symbol_latex("x_0"), "x_{0}");
        assert_eq!(symbol_latex("x0"), "x_{0}");
        assert_eq!(symbol_latex("omega0"), "\\omega_{0}");
        assert_eq!(symbol_latex("k_B"), "k_{B}");
        assert_eq!(symbol_latex("Tmax"), "\\mathrm{Tmax}");
        assert_eq!(symbol_latex("v_alpha"), "v_{\\alpha}");
    }

    #[test]
    fn numbers() {
        assert_eq!(number_latex(2.0, false), "2");
        assert_eq!(number_latex(0.987, true), "0{,}987");
        assert_eq!(number_latex(1.5e-7, true), "1{,}5 \\times 10^{-7}");
        assert_eq!(number_latex(3e8, false), "3 \\times 10^{8}");
        assert_eq!(number_latex(1e-9, false), "10^{-9}");
        assert_eq!(number_latex(-9.81, false), "-9.81");
        assert_eq!(number_latex(250000.0, false), "250000");
    }

    #[test]
    fn overrides_and_comma() {
        let mut o = LatexOptions {
            decimal_comma: true,
            ..Default::default()
        };
        o.symbols.insert("g".into(), "g_{\\mathrm{ef}}".into());
        let e = parse("0.987*2*pi*sqrt(x/g)").unwrap();
        assert_eq!(to_latex(&e, &o), "0{,}987 \\cdot 2 \\pi \\sqrt{\\frac{x}{g_{\\mathrm{ef}}}}");
    }
}
